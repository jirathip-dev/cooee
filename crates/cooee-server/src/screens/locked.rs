//! The two locked screens (#316): `GET /screens/pairing`, the app's bundled pairing steps, and
//! `GET /screens/reset-layout`, the one button that puts the default `layout.json` back.

use blocks::{Block, Button, ButtonStyle, Screen, Text};
use topcoat::{Result, context::Cx, router::route};

use crate::http::Answer;

const PAIRING: &str =
    include_str!("../../../../apple/Sources/CooeeUI/Default/screens/pairing.json");

/// `{type: screen, source: core, id, title, blocks}`: one `step` block per pairing step, as the app draws them.
#[route(GET "/screens/pairing")]
pub async fn pairing(_cx: &Cx) -> Result<Answer<Block>> {
    Ok(Answer(super::bundled(PAIRING).map(super::core)))
}

/// `{type: screen, source: core, id, title, blocks}`: a `reset` button posting `POST /layout/reset`.
#[route(GET "/screens/reset-layout")]
pub async fn reset_layout(_cx: &Cx) -> Result<Answer<Block>> {
    let why = Text::default()
        .id("why")
        .text("Puts the default layout back. The old layout.json is kept as layout.json.bak.");
    let reset = Button::new("Reset layout")
        .id("reset")
        .style(ButtonStyle::Destructive)
        .action("/layout/reset");
    let screen = Screen::default()
        .id("reset-layout")
        .title("Reset layout")
        .blocks(vec![
            Block::Text(Box::new(why)),
            Block::Button(Box::new(reset)),
        ]);
    Ok(Answer(Ok(super::core(screen))))
}
