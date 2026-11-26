use cooee_core::{Body, Item, Thread};

use super::{live::Live, waits};

#[derive(Clone)]
pub(super) struct Glance {
    pub(super) ask: Option<(String, String, String)>,
    pub(super) last: String,
    pub(super) speaker: String,
    pub(super) at: Option<u64>,
    pub(super) unseen: usize,
    pub(super) routine: String,
}

pub(super) fn glance(live: &Live) -> Glance {
    let items = &live.chat.items;
    let from = items.partition_point(|i| i.id <= live.meta.read.unwrap_or(0));
    let after = items.get(from..).unwrap_or_default().iter();
    let unseen = after.filter(|i| !matches!(i.body, Body::You { .. }));
    summary(&live.meta, items, unseen.count())
}

pub(super) fn summary(meta: &Thread, items: &[Item], unseen: usize) -> Glance {
    let ask = items.iter().find_map(|i| match &i.body {
        Body::Approval {
            tool,
            tapped: None,
            chosen: None,
            card,
            ..
        } if card.line.is_none() => Some((meta.room.clone(), i.speaker.clone(), tool.clone())),
        _ => None,
    });
    let last = items.last();
    let text = last.map_or("", |i| match &i.body {
        Body::Link { text, title, .. } if text.is_empty() => title,
        Body::You { text, .. }
        | Body::Agent { text, .. }
        | Body::Note { text }
        | Body::Frame { text, .. }
        | Body::Link { text, .. }
        | Body::Ask { text, .. }
        | Body::Handoff { text, .. }
        | Body::Ended { text } => text,
        Body::Tool { subject, .. } => subject,
        Body::Approval { tool, .. } => tool,
        Body::Plan { .. } => "plan",
        Body::Blocks(_) => "blocks",
        Body::Routine { name, .. } => name,
    });
    let waiting = items.iter().find_map(|i| match &i.body {
        Body::Approval { tool, .. } if waits(&i.body) => Some(format!("{tool} waiting")),
        _ => None,
    });
    Glance {
        ask,
        last: waiting
            .unwrap_or_else(|| text.lines().last().unwrap_or("").chars().take(80).collect()),
        at: last.map(|i| i.at),
        speaker: last.map(|i| i.speaker.clone()).unwrap_or_default(),
        unseen,
        routine: routine_of(meta, items),
    }
}

fn routine_of(meta: &Thread, items: &[Item]) -> String {
    let you = items.iter().rev().find_map(|i| match &i.body {
        Body::You { routine, .. } => Some(routine),
        _ => None,
    });
    let mark = meta.routine.as_ref().or_else(|| you?.as_ref());
    mark.map(|m| m.name.clone()).unwrap_or_default()
}
