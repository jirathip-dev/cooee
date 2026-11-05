//! cooee's HTTP server: serves the web UI (`cooee-web`) on localhost. Live events, pairing and push come later.

#![forbid(unsafe_code)]

use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
};

/// Serves the web UI on `127.0.0.1:port` until Ctrl+C or `SIGTERM`. `ready` gets the bound address first (port 0
/// picks a free one).
///
/// # Errors
///
/// Returns `Err` if the runtime can't start, the port can't be bound or accepting a connection fails.
pub fn serve(port: u16, ready: impl FnOnce(SocketAddr)) -> io::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
            ready(listener.local_addr()?);
            topcoat::serve(listener, cooee_web::router()).await
        })
}
