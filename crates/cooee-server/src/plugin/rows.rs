use std::{collections::HashSet, io, sync::Arc, time::Duration};

use cooee_core::plugins::rows::{Row, Rows};
use futures_util::StreamExt;

use super::{Heard, call, events};
use crate::Hub;

const AGAIN: Duration = Duration::from_secs(2);

/// Follows each installed plugin's rows for `hub` (drover#1571), at the paths its manifest's `[threads]` names: the
/// open rows on a first connect, then its stream from the last row applied, again after it ends.
pub fn follow_rows(hub: Arc<Hub>) {
    tokio::spawn(async move {
        let mut followed = HashSet::new();
        loop {
            for name in hub.row_plugins() {
                if followed.insert(name.clone()) {
                    tokio::spawn(follow_one(Arc::clone(&hub), name));
                }
            }
            tokio::time::sleep(AGAIN).await;
        }
    });
}

async fn follow_one(hub: Arc<Hub>, name: String) {
    loop {
        if let Err(e) = follow_once(&hub, &name).await {
            cooee_core::note!("cooee: plugin {name}: its rows: {e}");
        }
        tokio::time::sleep(AGAIN).await;
    }
}

async fn off<T: Send + 'static>(
    f: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(io::Error::other)?
}

async fn follow_once(hub: &Arc<Hub>, name: &str) -> io::Result<()> {
    let threads = hub.with_rows(name).and_then(|m| m.threads);
    let (Some(p), Some(t)) = (hub.backend(name), threads) else {
        return Ok(());
    };
    let at = match hub.row_cursor(name) {
        Some(at) => at,
        None => {
            let (b, path) = (p.clone(), format!("{}?open=1", t.rows));
            let rows: Rows = off(move || {
                let r = call(&b, "GET", &path, &[], &[], Duration::from_secs(10))?;
                Ok(serde_json::from_slice(&r.bytes()?)?)
            })
            .await?;
            let (h, n) = (Arc::clone(hub), name.to_owned());
            off(move || {
                for row in rows.rows {
                    h.apply_row(&n, row, false);
                }
                h.keep_row_cursor(&n, rows.seq).map(|()| rows.seq)
            })
            .await?
        }
    };
    let (b, path) = (p.clone(), t.stream.clone());
    let reply = off(move || {
        let id = at.to_string();
        let headers = [
            ("Accept", "text/event-stream"),
            ("Last-Event-ID", id.as_str()),
        ];
        call(&b, "GET", &path, &headers, &[], Duration::from_secs(5))
    })
    .await?;
    if !reply.streams() {
        return Err(io::Error::other(format!(
            "its stream answered {}",
            reply.status
        )));
    }
    let mut heard = Box::pin(events(reply)?);
    let mut at = at;
    while let Some(h) = heard.next().await {
        let Heard::Event(e) = h else {
            continue;
        };
        let row: Row = match serde_json::from_str(&e.data) {
            Ok(row) => row,
            Err(e) => {
                cooee_core::note!("cooee: plugin {name}: a row it sent: {e}");
                continue;
            }
        };
        let (seq, loud) = (row.seq, row.seq > at);
        let (h, n) = (Arc::clone(hub), name.to_owned());
        off(move || {
            h.apply_row(&n, row, loud);
            if loud {
                h.keep_row_cursor(&n, seq)
            } else {
                Ok(())
            }
        })
        .await?;
        at = at.max(seq);
    }
    Ok(())
}
