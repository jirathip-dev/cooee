use std::{sync::Arc, time::Duration};

use blocks::{
    Action, Block, Button, FieldValue, Fields, Form, Screen, Submit, Tap, Text, TextField, Toast,
    Tone,
};
use cooee_core::{Claim, Devices, Peek, device_key};
use serde::Deserialize;
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{Body, RouterBuilder, content::Json, parse_query_params, response::Response, route},
};

use crate::{
    http::{Answer, Paired, now, plain},
    live::{Live, Source, Wake, Who},
    pairing::{
        Assertion, Tried, ask, claim, claim_cookie, collect, cookie, host, rp_id, sign_in, store,
        try_code,
    },
};

const PATH: &str = "/screens/pair";

const OPEN: &str = "/screens/sessions";

enum Caller {
    Nobody { host: String, code: String },
    Asking(Peek),
    Paired(String),
}

#[derive(Deserialize)]
struct Code {
    code: Option<String>,
}

async fn caller(cx: &Cx) -> Result<Caller> {
    let host = rp_id(host(cx)).to_owned();
    if try_request_context::<Paired>(cx).is_some() {
        return Ok(Caller::Paired(host));
    }
    if let Some(claim) = claim(cx) {
        return Ok(Caller::Asking(
            store(cx, move |d| d.peek(&claim, now())).await?,
        ));
    }
    let q: Option<Code> = parse_query_params(cx).ok();
    let code = q.and_then(|q| q.code).unwrap_or_default();
    Ok(Caller::Nobody { host, code })
}

/// `{type: screen, source: core, id: pair}`, open to anyone and drawn for who asks (`?code=` fills the code).
#[route(GET "/screens/pair")]
pub async fn pair(cx: &Cx) -> Result<Answer<Block>> {
    Ok(Answer(Ok(screen(&caller(cx).await?, now()))))
}

pub(super) fn source(path: &str, who: &Who) -> Option<Source> {
    let Who::Asking { claim, data } = who else {
        return None;
    };
    (path == PATH).then(|| {
        let (claim, devices) = (claim.clone(), Devices::new(data));
        Source {
            wake: Wake::Every(Duration::from_secs(1)),
            build: Box::new(move |_| {
                let peek = devices.peek(&claim, now()).map_err(crate::http::refused)?;
                Ok(screen(&Caller::Asking(peek), now()))
            }),
        }
    })
}

fn text(id: &str, text: impl Into<String>) -> Block {
    Block::Text(Box::new(Text::default().id(id).text(text.into())))
}

fn button(label: &str, act: &str) -> Block {
    Block::Button(Box::new(Button::new(label).id(act).action(act)))
}

fn key(mode: &str) -> Block {
    let instead = "Pair with the code instead";
    device_key("key", mode, mode, Fields::new(), instead)
}

fn screen(c: &Caller, now: u64) -> Block {
    let (title, blocks) = match c {
        Caller::Nobody { host, code } => {
            let mut field = TextField::new("code").label("Six-digit code");
            if !code.is_empty() {
                field = field.value(FieldValue::Text(code.clone()));
            }
            let form = Form::new(vec![field.into()])
                .id("code")
                .action("code")
                .submit(Submit {
                    label: "Pair".into(),
                    action: "code".into(),
                    ..Submit::default()
                });
            let blocks = vec![
                Block::Form(Box::new(form)),
                button("Ask to pair", "ask"),
                key("signin"),
            ];
            (format!("Pair with {host}"), blocks)
        }
        Caller::Asking(Peek::Waiting(word, until)) => {
            let left = until.saturating_sub(now).div_ceil(1000);
            let blocks = vec![
                text("word", format!("Check word: {word}")),
                text(
                    "left",
                    format!("Runs out in {}:{:02}", left / 60, left % 60),
                ),
                text(
                    "how",
                    "Approve it on a paired device, or run `cooee pair approve` on the server.",
                ),
                button("Cancel", "cancel"),
            ];
            ("Waiting for approval".to_owned(), blocks)
        }
        Caller::Asking(Peek::Approved(by)) => {
            let blocks = vec![
                text("by", format!("{by} approved this device.")),
                button("Continue", "collect"),
            ];
            ("Approved".to_owned(), blocks)
        }
        Caller::Asking(peek) => {
            let why = match peek {
                Peek::Declined(by) => format!("{by} said no."),
                _ => "The request ran out.".to_owned(),
            };
            let blocks = vec![text("why", why), button("Ask again", "ask")];
            ("Not paired".to_owned(), blocks)
        }
        Caller::Paired(host) => {
            let open = Tap {
                open: Some(OPEN.to_owned()),
                ..Tap::default()
            };
            let blocks = vec![
                text("paired", format!("Paired with {host}")),
                key("enroll"),
                Block::Button(Box::new(Button::new("Open cooee").id("open").tap(open))),
            ];
            ("Paired".to_owned(), blocks)
        }
    };
    let screen = Screen::default().id("pair").title(title).blocks(blocks);
    crate::http::core(screen)
}

fn answer(a: &blocks::Answer, cookies: &[String]) -> Result<Response> {
    let mut r = Response::builder().header("Content-Type", "application/json");
    for c in cookies {
        r = r.header("Set-Cookie", c);
    }
    Ok(r.body(Body::from(serde_json::to_vec(a)?))?)
}

fn open(to: &str) -> blocks::Answer {
    blocks::Answer {
        open: Some(to.to_owned()),
        ..blocks::Answer::default()
    }
}

fn bad(text: &str) -> Result<Response> {
    let toast = Toast::new(text).tone(Tone::Fail);
    let a = blocks::Answer {
        toast: Some(toast),
        ..blocks::Answer::default()
    };
    answer(&a, &[])
}

/// `POST /screens/pair/action`, open to anyone: `code`, `ask`, `cancel`, `collect` or `signin`, answering an `Answer`.
#[route(POST "/screens/pair/action")]
pub async fn action(cx: &Cx, Json(a): Json<Action>) -> Result<Response> {
    let value = |k: &str| a.values.get(k).and_then(Value::as_str).unwrap_or_default();
    match a.action.as_str() {
        "code" => {
            let code: String = value("code").chars().filter(char::is_ascii_digit).collect();
            match try_code(cx, &code).await? {
                Tried::Paired(token) => answer(&open(OPEN), &[cookie(cx, &token)]),
                Tried::Wrong { left } => bad(&format!("That code is wrong: {left} tries left.")),
                Tried::Expired => bad("That code ran out or was used: run `cooee pair` again."),
                Tried::Limited { wait } => bad(&format!("Too many tries: wait {wait} s.")),
            }
        }
        "ask" => {
            let (_, claim) = ask(cx).await?;
            answer(&open(PATH), &[claim_cookie(cx, Some(&claim))])
        }
        "cancel" => {
            if let Some(claim) = claim(cx) {
                app_context::<Arc<Live>>(cx).hand_over(&claim);
                store(cx, move |d| d.cancel(&claim)).await?;
            }
            answer(&open(PATH), &[claim_cookie(cx, None)])
        }
        "collect" => {
            let Some(claim) = claim(cx) else {
                return bad("The request ran out: ask again.");
            };
            match collect(cx, claim.clone()).await? {
                Claim::Paired(token) => {
                    app_context::<Arc<Live>>(cx).hand_over(&claim);
                    let cookies = [cookie(cx, &token), claim_cookie(cx, None)];
                    answer(&open(OPEN), &cookies)
                }
                Claim::Waiting => bad("Not approved yet."),
                Claim::Declined(by) => bad(&format!("{by} said no.")),
                Claim::Gone => bad("The request ran out: ask again."),
            }
        }
        "signin" => {
            let Ok(signed) = serde_json::from_value::<Assertion>(Value::Object(a.values.clone()))
            else {
                return bad("That sign-in isn't readable.");
            };
            match sign_in(cx, signed).await? {
                Ok(token) => answer(&open(OPEN), &[cookie(cx, &token)]),
                Err(why) => bad(why),
            }
        }
        _ => plain(404, "no such action"),
    }
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(pair).route(action)
}
