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
    Hub, Layouts, Logins, Manager, connectors, editor,
    gate::{Gate, Gzip, Timing, names},
    http::{Data, profiles, provider_name, recipe},
    hub, layout, look, pairing,
    passkey::Unlocks,
    plugin, routes, routines, screens, themes,
};

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
            (&data, config.as_deref(), &list, &hub.mcp()?),
            ask,
            PathBuf::new(),
            (empty, false),
        )
    })
}

fn runner(
    hub: &Arc<Hub>,
    logins: Arc<Logins>,
    data: &std::path::Path,
    (config, hooks): (
        Option<&std::path::Path>,
        Option<cooee_core::routines::Hooks>,
    ),
) -> Arc<routines::Runner> {
    let (dir, config) = (data.to_owned(), config.map(ToOwned::to_owned));
    let (data, settings) = (dir.clone(), config.clone());
    let cheap = Arc::clone(&logins);
    let cheapest: routines::Cheapest = Box::new(move |agent| cheap.cheapest(agent));
    let start: routines::Start = Box::new(move |hub, who| {
        let list = profiles(hub, (&data, config.as_deref()));
        let (agent, folder, mut recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list, &hub.mcp()?),
            (&who.agent, Some(&who.account), &who.profile),
            who.folder.clone(),
            ((who.model.clone(), who.effort.clone()), false),
        )?;
        if recipe.machine.is_empty() {
            recipe.machine.clone_from(&who.machine);
        }
        let name = provider_name(&agent, &logins.statuses());
        Ok((agent, name, folder, recipe))
    });
    Arc::new(routines::Runner::new(
        Arc::clone(hub),
        (settings.as_deref(), &dir),
        (start, cheapest),
        hooks,
    ))
}

/// The threads' API over `hub`, its routines' `runner`, the providers' `logins`, the plugin `manager` and pairing, behind the gate: requests must name `bind`, localhost
/// or one of `allowed_hosts`; devices are paired in `data`.
#[must_use]
pub fn router(
    (hub, runner): (Arc<Hub>, Arc<routines::Runner>),
    (logins, manager, machines, config_all): (
        Arc<Logins>,
        Arc<Manager>,
        Arc<Machines>,
        Arc<Config>,
    ),
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
    screens::routes(editor::routes(themes::routes(routes::routes(
        Router::builder()
            .layer(Timing)
            .layer(Gzip)
            .layer(gate)
            .layer(BodyLimit::max(64 << 20).at("/threads")),
    ))))
    .route(layout::get)
    .route(layout::events)
    .route(layout::reset)
    .app_context(layouts)
    .app_context(hub)
    .app_context(runner)
    .app_context(Arc::new(Unlocks::default()))
    .app_context(Arc::new(connectors::Connectors::new(
        data,
        connectors::KEYCHAIN,
    )))
    .app_context(logins)
    .app_context(manager)
    .app_context(machines)
    .app_context(config_all)
    .app_context(Data(data.to_owned(), config.map(ToOwned::to_owned)))
    .app_context(Arc::new(pairing::Pairing::default()))
    .build()
}

fn retain(config: &Config, kept: std::path::PathBuf) {
    let history = config.history;
    std::thread::spawn(move || {
        loop {
            match cooee_core::Store::open(&kept).and_then(|s| s.retain(hub::now(), &history)) {
                Ok(0) => {}
                Ok(n) => cooee_core::note!("cooee: history: archived or trimmed {n} items"),
                Err(e) => cooee_core::note!("cooee: history: {e}"),
            }
            cooee_core::clock::sleep(Duration::from_hours(1));
        }
    });
}

fn housekeep(config: &Config, data: &std::path::Path) -> io::Result<()> {
    let logs = (config.logs.clone(), data.to_owned());
    std::thread::spawn(move || {
        loop {
            for name in cooee_core::logs::FILES {
                if let Err(e) = logs.0.rotate(&logs.1.join(name)) {
                    cooee_core::note!("cooee: rotating {name}: {e}");
                }
            }
            cooee_core::clock::sleep(Duration::from_secs(600));
        }
    });
    let old = themes::Themes::new(config)?;
    std::thread::spawn(move || match old.migrate() {
        Ok(0) => {}
        Ok(freed) => cooee_core::note!(
            "cooee: themes: freed {freed} bytes ({} MB) of old clones",
            freed >> 20
        ),
        Err(e) => cooee_core::note!("cooee: themes: old clones: {e}"),
    });
    Ok(())
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
    let own = config.path.is_some() && config.path == Config::default_path(&config.instance);
    if let Some(dir) = config.dir.as_deref().filter(|d| own && d.is_dir()) {
        cooee_core::gitignore::write(dir)?;
    }
    let manager = Manager::new(config)?;
    plugin::check(&manager.tabs())?;
    let data = config
        .data_dir()
        .ok_or_else(|| io::Error::other("no data folder: set `data` or HOME"))?;
    housekeep(config, &data)?;
    cooee_core::perf::init(&data, config.perf.slow_ms)?;
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let (looks, refused) = look::Looks::load(config.dir.as_deref(), config.theme.as_deref());
    for why in refused {
        cooee_core::note!("cooee: refused {why}");
    }
    let theme = looks
        .themes
        .get(&looks.default)
        .cloned()
        .ok_or_else(|| io::Error::other(format!("no theme {}", looks.default)))?;
    let providers = cooee_core::Providers::load(config.providers.as_deref(), &shed);
    let logins = Logins::new(shed.clone(), providers.clone(), &data);
    let listener = std::net::TcpListener::bind((config.bind, config.port))?;
    listener.set_nonblocking(true)?;
    let hub = Hub::open(
        (shed, providers),
        &data,
        (config.roots.clone(), config.path.clone()),
        theme,
        manager.tabs(),
        config.apns.clone(),
        config.approval_expiry_minutes.saturating_mul(60_000),
    )?;
    retain(config, data.clone());
    let (daily, signins) = (Arc::clone(&hub), Arc::clone(&logins));
    std::thread::spawn(move || {
        loop {
            for (agent, account) in daily.stale(hub::now()) {
                let launch = signins.home(&agent, Some(&account));
                let probed = launch.and_then(|(_, l)| daily.probe(&agent, &account, &l));
                if let Err(e) = probed {
                    cooee_core::note!("cooee: models of {agent}: {e}");
                }
            }
            cooee_core::clock::sleep(Duration::from_hours(1));
        }
    });
    let runner = runner(
        &hub,
        Arc::clone(&logins),
        &data,
        (config.dir.as_deref(), config.hooks.clone()),
    );
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
            Arc::new(config.clone()),
        ),
        (&data, config.dir.as_deref()),
        config.bind,
        &config.allowed_hosts,
        Layouts::watch(config.dir.clone()).map_err(|b| io::Error::other(b.why))?,
    );
    let served = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::from_std(listener)?;
            let addr = listener.local_addr()?;
            hub.listening(format!("http://127.0.0.1:{}", addr.port()));
            manager.supervise(hub, format!("http://127.0.0.1:{}", addr.port()));
            routines::Runner::every(runner, Duration::from_secs(30))?;
            ready(addr);
            topcoat::serve(listener, router).await
        });
    manager.stop_all();
    served
}
