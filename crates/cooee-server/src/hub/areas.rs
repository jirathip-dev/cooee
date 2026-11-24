use super::{
    HubError, Seed, ask, desk, events, expire, folders, grants, mail, models, plugins, proposals,
    rooms, route, screen, servers, setup, start, surface,
};

pub(super) struct Areas {
    pub(super) ask: ask::State,
    pub(super) desk: desk::State,
    pub(super) events: events::State,
    pub(super) expire: expire::State,
    pub(super) folders: folders::State,
    pub(super) grants: grants::State,
    pub(super) mail: mail::State,
    pub(super) models: models::State,
    pub(super) plugins: plugins::State,
    pub(super) proposals: proposals::State,
    pub(super) rooms: rooms::State,
    pub(super) route: route::State,
    pub(super) screen: screen::State,
    pub(super) servers: servers::State,
    pub(super) setup: setup::State,
    pub(super) start: start::State,
    pub(super) surface: surface::State,
}

impl Areas {
    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {
        Ok(Self {
            ask: ask::State::default(),
            desk: desk::State::default(),
            events: events::State::default(),
            expire: expire::State::open(seed),
            folders: folders::State::open(seed),
            grants: grants::State::open(seed)?,
            mail: mail::State::default(),
            models: models::State::open(seed),
            plugins: plugins::State::open(seed),
            proposals: proposals::State::default(),
            rooms: rooms::State::default(),
            route: route::State::default(),
            screen: screen::State::default(),
            servers: servers::State::default(),
            setup: setup::State::default(),
            start: start::State::default(),
            surface: surface::State::default(),
        })
    }
}
