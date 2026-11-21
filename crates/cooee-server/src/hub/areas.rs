use super::{
    HubError, Seed, ask, desk, events, expire, folders, mail, mcp, models, plugins, rooms, screen,
    setup, start, surface,
};

pub(super) struct Areas {
    pub(super) ask: ask::State,
    pub(super) desk: desk::State,
    pub(super) events: events::State,
    pub(super) expire: expire::State,
    pub(super) folders: folders::State,
    pub(super) mail: mail::State,
    pub(super) mcp: mcp::State,
    pub(super) models: models::State,
    pub(super) plugins: plugins::State,
    pub(super) rooms: rooms::State,
    pub(super) screen: screen::State,
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
            mail: mail::State::default(),
            mcp: mcp::State::open(seed)?,
            models: models::State::open(seed),
            plugins: plugins::State::open(seed),
            rooms: rooms::State::default(),
            screen: screen::State::default(),
            setup: setup::State::default(),
            start: start::State::default(),
            surface: surface::State::default(),
        })
    }
}
