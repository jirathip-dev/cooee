use std::{collections::HashSet, io, sync::Arc, time::Duration};

use cooee_core::{
    Plugin,
    plugins::rows::{Row, Rows},
};
use futures_util::StreamExt;

use super::{Heard, call, events};
use crate::Hub;

const AGAIN: Duration = Duration::from_secs(2);

/// Follows each installed plugin's rows for `hub` at the paths its manifest's `[threads]` names.
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
        let (asked, named) = (Arc::clone(&hub), name.clone());
        let offered = off(move || {
            asked.offer(&named);
            Ok(())
        });
        if let Err(e) = offered.await.and(follow_once(&hub, &name).await) {
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

async fn first_rows(hub: &Arc<Hub>, name: &str, backend: Plugin, path: String) -> io::Result<u64> {
    let rows: Rows = off(move || {
        let reply = call(&backend, "GET", &path, &[], &[], Duration::from_secs(10))?;
        Ok(serde_json::from_slice(&reply.bytes()?)?)
    })
    .await?;
    let (hub, name) = (Arc::clone(hub), name.to_owned());
    off(move || {
        for row in rows.rows {
            hub.apply_row(&name, row, false);
        }
        hub.keep_row_cursor(&name, rows.seq).map(|()| rows.seq)
    })
    .await
}

async fn follow_once(hub: &Arc<Hub>, name: &str) -> io::Result<()> {
    let threads = hub.with_rows(name).and_then(|m| m.threads);
    let (Some(backend), Some(paths)) = (hub.backend(name), threads) else {
        return Ok(());
    };
    let mut at = if let Some(at) = hub.row_cursor(name) {
        at
    } else {
        first_rows(hub, name, backend.clone(), format!("{}?open=1", paths.rows)).await?
    };
    let reply = off(move || {
        let id = at.to_string();
        let headers = [
            ("Accept", "text/event-stream"),
            ("Last-Event-ID", id.as_str()),
        ];
        call(
            &backend,
            "GET",
            &paths.stream,
            &headers,
            &[],
            Duration::from_secs(5),
        )
    })
    .await?;
    if !reply.streams() {
        return Err(io::Error::other(format!(
            "its stream answered {}",
            reply.status
        )));
    }
    let mut heard = Box::pin(events(reply)?);
    while let Some(next) = heard.next().await {
        let Heard::Event(event) = next else {
            continue;
        };
        let row: Row = match serde_json::from_str(&event.data) {
            Ok(row) => row,
            Err(e) => {
                cooee_core::note!("cooee: plugin {name}: a row it sent: {e}");
                continue;
            }
        };
        let (seq, loud) = (row.seq, row.seq > at);
        let (hub, name) = (Arc::clone(hub), name.to_owned());
        off(move || {
            hub.apply_row(&name, row, loud);
            if loud {
                hub.keep_row_cursor(&name, seq)
            } else {
                Ok(())
            }
        })
        .await?;
        at = at.max(seq);
    }
    Ok(())
}
