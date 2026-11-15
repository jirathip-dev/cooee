//! The router over every route area behind the gate, and the server that runs it.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use cooee_core::{Config, Devices};
use cooee_machines::Machines;
use topcoat::router::{BodyLimit, Router};

use crate::{
    Hub, Layouts, Logins, Manager,
    gate::{Gate, Timing, names},
    http::{Data, profiles, provider_name, recipe},
    hub, layout, look, pairing,
    passkey::Unlocks,
    plugin, routes, routines,
};

/// A room member's session starts as its persona's chat does (#154).
fn starter(
    logins: Arc<Logins>,
    data: &std::path::Path,
    config: Option<&std::path::Path>,
) -> hub::Starter {
    let (data, config) = (data.to_owned(), config.map(ToOwned::to_owned));
    Box::new(move |hub, persona| {
        let list = profiles(hub, (&data, config.as_deref()));
        let empty = (String::new(), String::new());
        let ask = ("", None, persona);
        recipe(
            &logins,
            (&data, config.as_deref(), &list),
            ask,
            PathBuf::new(),
            empty,
        )
    })
}

/// The routines' runner (#75): a `new_thread` run starts as the chat that made it did.
fn runner(
    hub: &Arc<Hub>,
    logins: Arc<Logins>,
    data: &std::path::Path,
    config: Option<&std::path::Path>,
) -> Arc<routines::Runner> {
    let (dir, config) = (data.to_owned(), config.map(ToOwned::to_owned));
    let data = dir.clone();
    let start: routines::Start = Box::new(move |hub, who| {
        let list = profiles(hub, (&data, config.as_deref()));
        let (agent, folder, mut recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&who.agent, Some(&who.account), &who.profile),
            who.folder.clone(),
            (who.model.clone(), who.effort.clone()),
        )?;
        if recipe.machine.is_empty() {
            recipe.machine.clone_from(&who.machine);
        }
        let name = provider_name(&agent, &logins.statuses());
        Ok((agent, name, folder, recipe))
    });
    Arc::new(routines::Runner::new(Arc::clone(hub), &dir, start))
}

/// The threads' API over `hub`, its routines' `runner`, the providers' `logins`, the plugin `manager` and pairing, behind the gate: requests must name `bind`, localhost
/// or one of `allowed_hosts`; devices are paired in `data`.
#[must_use]
pub fn router(
    (hub, runner): (Arc<Hub>, Arc<routines::Runner>),
    (logins, manager, machines): (Arc<Logins>, Arc<Manager>, Arc<Machines>),
    (data, config): (&std::path::Path, Option<&std::path::Path>),
    bind: IpAddr,
    allowed_hosts: &[String],
    layouts: Arc<Layouts>,
) -> Router {
    hub.set_starter(starter(Arc::clone(&logins), data, config));
    let gate = Gate {
        names: names(bind, allowed_hosts),
        devices: Devices::new(data),
        hub: Arc::clone(&hub),
    };
    routes::routes(
        Router::builder()
            .layer(Timing)
            .layer(gate)
            // A message's photos come as base64.
            .layer(BodyLimit::max(64 << 20).at("/threads")),
    )
    .route(layout::get)
    .route(layout::events)
    .route(layout::reset)
    .app_context(layouts)
    .app_context(hub)
    .app_context(runner)
    .app_context(Arc::new(Unlocks::default()))
    .app_context(logins)
    .app_context(manager)
    .app_context(machines)
    .app_context(Data(data.to_owned(), config.map(ToOwned::to_owned)))
    .app_context(Arc::new(pairing::Pairing::default()))
    .build()
}

/// Opens the threads (`config`'s data folder and shed) and the default theme (for push wording), and serves the API
/// on `bind:port` until Ctrl+C or `SIGTERM`. `ready` gets the bound address first (port 0 picks a free one).
///
/// # Errors
///
/// Returns `Err` if `bind` isn't loopback and `allowed_hosts` is empty, a plugin is misconfigured (no token, …),
/// the store can't be opened, the runtime can't start, the port can't be bound or accepting a connection fails.
pub fn serve(config: &Config, ready: impl FnOnce(SocketAddr)) -> io::Result<()> {
    if !config.bind.is_loopback() && config.allowed_hosts.is_empty() {
        return Err(io::Error::other(format!(
            "listening on {} needs `allowed_hosts` (the names the phone uses, e.g. the Tailscale name)",
            config.bind
        )));
    }
    let manager = Manager::new(config)?;
    plugin::check(&manager.tabs())?;
    let data = config
        .data_dir()
        .ok_or_else(|| io::Error::other("no data folder: set `data` or HOME"))?;
    cooee_core::perf::init(&data, config.perf.slow_ms)?;
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let (looks, refused) = look::Looks::load(config.dir.as_deref(), config.theme.as_deref());
    for why in refused {
        eprintln!("cooee: refused {why}");
    }
    let theme = looks.themes[&looks.default].clone();
    let logins = Logins::new(
        shed.clone(),
        cooee_core::Providers::new(&config.providers),
        &data,
    );
    let hub = Hub::open(
        shed,
        &data,
        (config.roots.clone(), config.path.clone()),
        theme,
        manager.tabs(),
        config.apns.clone(),
    )?;
    // The models' daily refresh (#193): each agent and account seen a day ago or more is asked again.
    let (daily, signins) = (Arc::clone(&hub), Arc::clone(&logins));
    std::thread::spawn(move || {
        loop {
            for (agent, account) in daily.stale(hub::now()) {
                let launch = signins.home(&agent, Some(&account));
                let probed = launch.and_then(|(_, l)| daily.probe(&agent, &account, &l));
                if let Err(e) = probed {
                    eprintln!("cooee: models of {agent}: {e}");
                }
            }
            std::thread::sleep(Duration::from_hours(1));
        }
    });
    let runner = runner(&hub, Arc::clone(&logins), &data, config.dir.as_deref());
    let router = router(
        (Arc::clone(&hub), Arc::clone(&runner)),
        (
            logins,
            Arc::clone(&manager),
            Arc::new(Machines::new(
                config.shed.clone(),
                config.shed_dir.clone(),
                cooee_machines::local_name(),
            )),
        ),
        (&data, config.dir.as_deref()),
        config.bind,
        &config.allowed_hosts,
        Layouts::watch(config.dir.clone()),
    );
    let served = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind((config.bind, config.port)).await?;
            let addr = listener.local_addr()?;
            hub.listening(format!("http://127.0.0.1:{}", addr.port()));
            manager.supervise(hub, format!("http://127.0.0.1:{}", addr.port()));
            routines::Runner::every(runner, Duration::from_secs(30));
            ready(addr);
            topcoat::serve(listener, router).await
        });
    manager.stop_all();
    served
}
