use std::{
    collections::{HashMap, VecDeque},
    io,
    mem::discriminant,
    pin::pin,
    slice,
    sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use blocks::{Block, Op};
use cooee_core::Plugin;
use futures_util::{
    Stream, StreamExt,
    future::{Either, select},
    stream,
};
use tokio::sync::broadcast;

use super::{Heard, Reply, Sse, call, events};

const KEPT_FRAMES: usize = 64;

static FEEDS: LazyLock<Mutex<HashMap<String, Arc<Feed>>>> = LazyLock::new(Mutex::default);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Feed {
    tx: broadcast::Sender<Sse>,
    kept: Mutex<Kept>,
}

#[derive(Default)]
struct Kept {
    last: Option<Sse>,
    screen: Option<Block>,
    marks: Vec<Op>,
    tail: VecDeque<Sse>,
}

impl Kept {
    fn take(&mut self, e: &Sse) {
        let ops = serde_json::from_str::<serde_json::Value>(&e.data)
            .ok()
            .and_then(|mut v| serde_json::from_value::<Vec<Op>>(v.get_mut("ops")?.take()).ok());
        let mut screen = self.screen.take().filter(|_| ops.is_some());
        for op in ops.iter().flatten() {
            match op {
                Op::Screen { screen: s } => screen = Some(s.clone()),
                Op::Badge { .. } | Op::Dot { .. } => {
                    self.marks.retain(|m| discriminant(m) != discriminant(op));
                    self.marks.push(op.clone());
                }
                _ => {
                    screen = screen.and_then(|mut b| {
                        blocks::apply(&mut b, slice::from_ref(op)).ok().map(|()| b)
                    });
                }
            }
        }
        self.screen = screen;
        if self.screen.is_some() {
            self.tail.clear();
        } else {
            if self.tail.len() >= KEPT_FRAMES {
                self.tail.pop_front();
            }
            self.tail.push_back(e.clone());
        }
        self.last = Some(e.clone());
    }

    fn replay(&self) -> Vec<Sse> {
        let (Some(screen), Some(last)) = (&self.screen, &self.last) else {
            return self.tail.iter().cloned().collect();
        };
        let ops = std::iter::once(Op::Screen {
            screen: screen.clone(),
        });
        let ops: Vec<Op> = ops.chain(self.marks.iter().cloned()).collect();
        let mut data = serde_json::from_str::<serde_json::Value>(&last.data).unwrap_or_default();
        if let (Some(o), Ok(ops)) = (data.as_object_mut(), serde_json::to_value(ops)) {
            o.insert("ops".to_owned(), ops);
        }
        vec![Sse {
            id: last.id.clone(),
            event: last.event.clone(),
            data: data.to_string(),
        }]
    }
}

impl Feed {
    fn join(&self) -> impl Stream<Item = Sse> + Send + use<> {
        let kept = lock(&self.kept);
        let rx = self.tx.subscribe();
        let live = stream::unfold(
            rx,
            |mut rx| async move { Some((rx.recv().await.ok()?, rx)) },
        );
        stream::iter(kept.replay()).chain(live)
    }

    fn publish(&self, e: Sse) {
        let mut kept = lock(&self.kept);
        kept.take(&e);
        drop(self.tx.send(e));
    }

    fn last_id(&self) -> String {
        let kept = lock(&self.kept);
        kept.last
            .as_ref()
            .and_then(|e| e.id.clone())
            .unwrap_or_default()
    }
}

fn feed_key(p: &Plugin, target: &str) -> String {
    format!("{}{target}", p.url)
}

/// One more client of `p`'s event stream at `target` when another client already follows it: the kept screen
/// (or the frames since the stream opened), then each event as the backend sends it.
pub fn follow(p: &Plugin, target: &str) -> Option<impl Stream<Item = Sse> + Send + use<>> {
    lock(&FEEDS).get(&feed_key(p, target)).map(|f| f.join())
}

/// `reply`, `p`'s event stream at `target`, shared with every later client of it (`follow`): one backend
/// connection, opened again at once when it ends after an event, closed when its last client leaves. A client that
/// falls `KEPT_FRAMES` behind is dropped.
///
/// # Errors
/// When the connection can't join the async runtime.
pub fn share(
    p: Plugin,
    target: String,
    reply: Reply,
) -> io::Result<impl Stream<Item = Sse> + Send + 'static> {
    let mut heard = Box::pin(events(reply)?);
    let key = feed_key(&p, &target);
    let mut feeds = lock(&FEEDS);
    if let Some(f) = feeds.get(&key) {
        return Ok(f.join());
    }
    let (tx, _) = broadcast::channel(KEPT_FRAMES);
    let feed = Arc::new(Feed {
        tx,
        kept: Mutex::default(),
    });
    let first = feed.join();
    feeds.insert(key.clone(), Arc::clone(&feed));
    drop(feeds);
    tokio::spawn(async move {
        let mut fresh = false;
        loop {
            let next = match select(heard.next(), pin!(feed.tx.closed())).await {
                Either::Left((next, _)) => next,
                Either::Right(_) if feed.tx.receiver_count() > 0 => continue,
                Either::Right(_) => {
                    let mut feeds = lock(&FEEDS);
                    if feed.tx.receiver_count() == 0 {
                        feeds.remove(&key);
                        return;
                    }
                    continue;
                }
            };
            match next {
                Some(Heard::Event(e)) => {
                    fresh = true;
                    feed.publish(e);
                }
                Some(Heard::Beat) => {}
                None if fresh => {
                    fresh = false;
                    let (p, target, id) = (p.clone(), target.clone(), feed.last_id());
                    let again = tokio::task::spawn_blocking(move || {
                        let headers = [("Accept", "text/event-stream"), ("Last-Event-ID", &id)];
                        call(&p, "GET", &target, &headers, &[], Duration::from_secs(5))
                    });
                    match again.await.ok().and_then(Result::ok).filter(Reply::streams) {
                        Some(r) => match events(r) {
                            Ok(h) => heard = Box::pin(h),
                            Err(_) => break,
                        },
                        None => break,
                    }
                }
                None => break,
            }
        }
        lock(&FEEDS).remove(&key);
    });
    Ok(first)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    use super::*;

    const SCREEN: &str = r#"{"op":"screen","screen":{"type":"screen","id":"hello","blocks":[{"type":"stat","id":"waves","value":"0"}]}}"#;

    fn screens(
        name: &str,
        said: impl Fn(usize, &mut TcpStream) + Send + 'static,
    ) -> (Plugin, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (heads, rx) = mpsc::channel();
        thread::spawn(move || {
            for (n, mut s) in listener.incoming().flatten().enumerate() {
                let mut head = [0; 2048];
                let got = s.read(&mut head).unwrap();
                heads
                    .send(String::from_utf8_lossy(&head[..got]).into_owned())
                    .unwrap();
                let open = format!(
                    "HTTP/1.0 200 OK\r\nContent-Type: text/event-stream\r\n\r\nid: {n}\ndata: {{\"seq\":{n},\"ops\":[{SCREEN}]}}\n\n"
                );
                s.write_all(open.as_bytes()).unwrap();
                said(n, &mut s);
            }
        });
        let p = Plugin {
            name: name.to_owned(),
            label: String::new(),
            url,
            module: String::new(),
            token: "token-0123456789abcdef".to_owned(),
        };
        (p, rx)
    }

    fn ops(e: &Sse) -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&e.data).unwrap()["ops"].clone()
    }

    #[tokio::test]
    async fn three_clients_of_a_screen_share_one_backend_stream_and_a_late_one_gets_the_kept_screen()
     {
        let (wave, waved) = mpsc::channel::<()>();
        let waved = Mutex::new(waved);
        let (p, heads) = screens("shared", move |_, s| {
            lock(&waved).recv().unwrap();
            let merge = r#"data: {"seq":1,"ops":[{"op":"merge","id":"waves","fields":{"value":"1"}},{"op":"badge","count":2}]}"#;
            s.write_all(format!("{merge}\n\n").as_bytes()).unwrap();
            while s.read(&mut [0; 64]).is_ok_and(|n| n > 0) {}
        });
        let target = "/screens/hello/stream";
        assert!(follow(&p, target).is_none());
        let reply = call(&p, "GET", target, &[], &[], Duration::from_secs(5)).unwrap();
        let mut clients = vec![share(p.clone(), target.to_owned(), reply).unwrap().boxed()];
        assert_eq!(ops(&clients[0].next().await.unwrap())[0]["op"], "screen");
        for _ in 0..2 {
            let mut c = follow(&p, target).unwrap().boxed();
            assert_eq!(ops(&c.next().await.unwrap())[0]["op"], "screen");
            clients.push(c);
        }
        wave.send(()).unwrap();
        for c in &mut clients {
            assert_eq!(ops(&c.next().await.unwrap())[0]["fields"]["value"], "1");
        }
        let late = Box::pin(follow(&p, target).unwrap()).next().await.unwrap();
        let kept = ops(&late);
        assert_eq!(
            (&kept[0]["screen"]["blocks"][0]["value"], &kept[1]),
            (
                &serde_json::json!("1"),
                &serde_json::json!({"op": "badge", "count": 2})
            )
        );
        assert_eq!(heads.try_iter().count(), 1);
    }

    #[tokio::test]
    async fn a_shared_stream_that_ends_is_opened_again_without_ending_its_clients_and_closes_with_its_last()
     {
        let (gone, left) = mpsc::channel();
        let (p, heads) = screens("again", move |n, s| {
            if n > 0 {
                while s.read(&mut [0; 64]).is_ok_and(|n| n > 0) {}
                gone.send(()).unwrap();
            }
        });
        let target = "/screens/hello/stream";
        let reply = call(&p, "GET", target, &[], &[], Duration::from_secs(5)).unwrap();
        let mut client = Box::pin(share(p.clone(), target.to_owned(), reply).unwrap());
        for n in ["0", "1"] {
            let e = client.next().await.unwrap();
            assert_eq!(
                (e.id.as_deref(), &ops(&e)[0]["op"]),
                (Some(n), &serde_json::json!("screen"))
            );
        }
        let again = heads.iter().nth(1).unwrap().to_ascii_lowercase();
        assert!(again.contains("last-event-id: 0\r\n"), "{again}");
        drop(client);
        tokio::task::spawn_blocking(move || left.recv_timeout(Duration::from_secs(5)))
            .await
            .unwrap()
            .unwrap();
        assert!(follow(&p, target).is_none());
    }
}
