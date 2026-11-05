//! #10 spike: cooee's chat screen in Topcoat. `/` the thread list, `/live` a thread fed by the fake shed, `/long` a
//! thread of 1,000 messages. Not production code.

mod shed;

use shed::{CHUNK, Shed, Status};
use std::sync::LazyLock;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, app_context},
    router::{Router, RouterBuilderDiscoverExt, route, page},
    runtime::{Event, RouterBuilderRuntimeExt, connected, procedure, signal},
    view::{Child, EmitToken, Unescaped, View, component, emit, live, view},
};

type S = &'static Shed;

/// The farm scheme as herdsman applies it: every token of `schemes/farm.json` on `:root`.
static SCHEME: LazyLock<String> = LazyLock::new(|| {
    let s: serde_json::Value = serde_json::from_str(include_str!("../theme/schemes/farm.json")).unwrap();
    let vars: String = s["tokens"].as_object().unwrap().iter().map(|(k, v)| format!("{k}:{};", v.as_str().unwrap())).collect();
    format!(":root{{{vars}color-scheme:{}}}", s["color-scheme"].as_str().unwrap())
});
/// The farm theme's sprite sheet, unchanged, as `<defs>` for `<use href="#row-…">`.
static SPRITES: &str = include_str!("../theme/themes/farm/sprites.svg");
static THEME: LazyLock<serde_json::Value> = LazyLock::new(|| serde_json::from_str(include_str!("../theme/themes/farm/theme.json")).unwrap());

fn row_sprite(kind: &str) -> String {
    THEME["row_sprites"][kind].as_str().unwrap_or("row-graze").to_owned()
}

#[component]
async fn sprite(kind: &str) -> Result<impl View> {
    // herdsman crops the 132 × 100 box to the 16 sprite px (8.25 each): 1.5 css px a pixel at 24 px.
    let href = format!("#{}", row_sprite(kind));
    Ok(view! { <svg class="rsp" viewBox="0 0 132 100" aria-hidden="true"><use href=(href)></use></svg> })
}

#[component]
async fn document(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
                <title>(title)</title>
                <style>(Unescaped::new_unchecked(SCHEME.as_str()))</style>
                <style>(Unescaped::new_unchecked(include_str!("style.css")))</style>
                topcoat::runtime::script()
            </head>
            <body>
                <svg width="0" height="0" style="position:absolute"><defs>(Unescaped::new_unchecked(SPRITES))</defs></svg>
                (child)
            </body>
        </html>
    })
}

#[page("/")]
async fn threads() -> Result<impl View> {
    let rows = [
        ("/live", "impl", "Fake shed: streaming reply", "w1999 · tool calls · approval"),
        ("/long", "done", "Long thread", "1,000 messages"),
        ("/long", "needs", "Needs you", "approve `git push`?"),
        ("/long", "idle", "ภาษาไทย", "สวัสดีครับ ที่นี่"),
    ];
    Ok(view! {
        document(title: "cooee",
            <header class="bar"><b>"cooee"</b><span class="mute">"threads"</span></header>
            <ul class="threads">
                for (href, kind, title, last) in rows {
                    <li><a href=(href)>sprite(kind: kind)<span class="t"><b>(title)</b><small>(last)</small></span></a></li>
                }
            </ul>
        )
    })
}

/// Sends the composer's text into the thread.
#[procedure]
async fn send(cx: &Cx, text: String) -> Result<usize> {
    let shed = *app_context::<S>(cx);
    let text = text.trim().to_owned();
    if text.is_empty() || text.len() > 8192 { return Ok(0) }
    let n = { let mut s = shed.sent.lock().unwrap(); s.push(text); s.len() };
    let _ = shed.tx.send(());
    Ok(n)
}

/// The approval row's option buttons.
#[procedure]
async fn choose(cx: &Cx, option: String) -> Result<bool> {
    Ok(app_context::<S>(cx).choose(option))
}

/// Restarts the fake shed's run (the measure scripts, and the thread's ↻).
#[procedure]
async fn restart(cx: &Cx) -> Result<bool> {
    app_context::<S>(cx).start();
    Ok(true)
}

#[route(POST "/api/start")]
async fn api_start(cx: &Cx) -> Result<&'static str> {
    app_context::<S>(cx).start();
    Ok("started")
}

#[component]
async fn composer(cx: &Cx) -> Result<impl View> {
    let draft = signal(cx, String::new);
    Ok(view! {
        <form class="composer" onsubmit="return false">
            <textarea id="composer" rows="1" placeholder="Message" :value=$(draft.get())
                @input=$(|e: Event| draft.set(e.target.value))></textarea>
            <button id="send" type="button" @click=$(async |_e| {
                let n = send(draft.get()).await;
                if n > 0 { draft.set("".to_owned()) }
            })>"Send"</button>
        </form>
    })
}

/// What the composer sent, as the thread's newest messages.
#[component]
async fn sent(cx: &Cx) -> Result<impl View> {
    Ok(live! {
        let shed = *app_context::<S>(cx);
        let mut rx = shed.tx.subscribe();
        let mut seen = usize::MAX;
        loop {
            let msgs = shed.sent.lock().unwrap().clone();
            if msgs.len() != seen {
                seen = msgs.len();
                emit! { for m in msgs { <li class="msg you">(m)</li> } }?;
            }
            if !connected(cx) { break Ok(EmitToken) }
            let _ = rx.recv().await;
        }
    })
}

/// One chunk of the streaming reply: it re-renders per token until it is full, then its region ends.
#[component]
async fn reply_chunk(cx: &Cx, c: usize) -> Result<impl View> {
    let shed = *app_context::<S>(cx);
    Ok(live! {
        let mut rx = shed.tx.subscribe();
        let mut seen = usize::MAX;
        loop {
            let (part, sealed) = shed.run.lock().unwrap().chunk(c);
            if part.len() != seen {
                seen = part.len();
                emit! { <span>(part)</span> }?;
            }
            if sealed || !connected(cx) { break Ok(EmitToken) }
            let _ = rx.recv().await;
        }
    })
}

#[page("/live")]
async fn live_thread(cx: &Cx) -> Result<impl View> {
    let shed = *app_context::<S>(cx);
    Ok(view! {
        document(title: "cooee · live",
            <main class="chat">
                <header class="bar"><a href="/">"‹"</a>sprite(kind: "impl")<b>"Fake shed"</b>
                    <button class="ghost" id="restart" @click=$(async |_e| { restart().await; })>"↻"</button></header>
                <ol class="msgs" id="msgs">
                    <li class="msg you">"Stream 2,000 tokens, call two tools and ask before pushing."</li>
                    <li class="msg agent">
                        // The reply text and the rows are separate regions: a token re-renders only the text.
                        (live! {
                            // A new chunk re-renders the list; each chunk streams in its own region (reply_chunk).
                            let mut rx = shed.tx.subscribe();
                            let mut seen = (u64::MAX, 0);
                            loop {
                                let now = { let r = shed.run.lock().unwrap(); (r.epoch, r.tokens / CHUNK) };
                                if now != seen {
                                    seen = now;
                                    emit! { <p class="reply" id="reply">#[key(c)] for c in 0..=now.1 { reply_chunk(c: c) }</p> }?;
                                }
                                if !connected(cx) { break Ok(EmitToken) }
                                let _ = rx.recv().await;
                            }
                        })
                        (live! {
                            let mut rx = shed.tx.subscribe();
                            let mut seen = u64::MAX;
                            loop {
                                let (v, tools, approval, done) = {
                                    let r = shed.run.lock().unwrap();
                                    (r.rows_v, r.tools.clone(), r.approval.clone(), r.done)
                                };
                                if v != seen {
                                    seen = v;
                                    emit! {
                                        <ul class="rows">
                                            for t in tools {
                                                <li class=(match t.status { Status::Pending => "tool pending", Status::Running => "tool running", Status::Done => "tool done" })>
                                                    sprite(kind: match t.status { Status::Pending => "queued", Status::Running => "impl", Status::Done => "done" })
                                                    <code>(t.name)</code>
                                                    <small>(match t.status { Status::Pending => "pending", Status::Running => "running", Status::Done => "done" })</small>
                                                </li>
                                            }
                                            if let Some(a) = approval {
                                                <li class="approval" id="approval">
                                                    sprite(kind: if a.chosen.is_some() { "done" } else { "needs" })
                                                    <span>(a.title)</span>
                                                    if let Some(c) = a.chosen {
                                                        <b class="chosen" id="chosen">(c)</b>
                                                    } else {
                                                        <span class="opts">
                                                            for o in a.options {
                                                                <button type="button" @click=$(async |_e| { choose(o.to_owned()).await; })>(o)</button>
                                                            }
                                                        </span>
                                                    }
                                                </li>
                                            }
                                            if done { <li class="tool done" id="done">"reply done"</li> }
                                        </ul>
                                    }?;
                                }
                                if !connected(cx) { break Ok(EmitToken) }
                                let _ = rx.recv().await;
                            }
                        })
                    </li>
                    sent()
                </ol>
                composer()
            </main>
        )
    })
}

/// 1,000 messages, both sides, some long, some Thai.
fn long_messages() -> Vec<(bool, String)> {
    let words = ["the", "build", "passed", "on", "staging", "สวัสดีครับ", "ที่นี่", "review", "found", "two", "issues", "merge"];
    (0..1000).map(|i| {
        let len = 4 + (i * 7919) % 60;
        let text: Vec<&str> = (0..len).map(|j| words[(i + j * 31) % words.len()]).collect();
        (i % 2 == 0, format!("#{i} {}", text.join(" ")))
    }).collect()
}

#[page("/long")]
async fn long_thread() -> Result<impl View> {
    Ok(view! {
        document(title: "cooee · long",
            <main class="chat">
                <header class="bar"><a href="/">"‹"</a>sprite(kind: "done")<b>"Long thread"</b></header>
                <ol class="msgs" id="msgs">
                    for (you, m) in long_messages() {
                        <li class=(if you { "msg you" } else { "msg agent" })>(m)</li>
                    }
                    sent()
                </ol>
                composer()
            </main>
        )
    })
}

#[tokio::main]
async fn main() {
    let tokens = std::env::var("TOKENS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000);
    let rate = std::env::var("RATE").ok().and_then(|v| v.parse().ok()).unwrap_or(50.0);
    let shed: S = Box::leak(Box::new(Shed::new(tokens, rate)));
    let router: Router = Router::builder()
        .discover()
        .assets(AssetBundle::load().expect("run `topcoat asset bundle` first (see README)"))
        .app_context(shed)
        .runtime()
        .build();
    topcoat::start(router).await.unwrap();
}
