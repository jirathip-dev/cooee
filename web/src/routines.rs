//! Settings › routines (#75), HTML-first in the mcp servers page's pattern: one row per routine, `name · when ·
//! where · last run · [pause|resume] [delete]`, and its edit (name, schedule, prompt). Routines are made in chat;
//! there is no "new" form. Every form posts and comes back here.

use cooee_core::routines::{Outcome, Paused, Place, Routine, Trigger};
use topcoat::{Result, context::Cx, view::View, view::component, view::view};

use crate::{age, document};

/// The page.
#[must_use]
pub fn routines_page(cx: &Cx, list: Vec<Routine>) -> impl View {
    view! { cx => routines(list: list) }
}

/// Where its runs post, and whether it is paused.
fn line(r: &Routine) -> String {
    let place = match &r.place {
        Place::Chat { .. } => "in its chat",
        Place::NewThread { .. } => "a new thread each run",
    };
    let last = match &r.last_run {
        None => "not run yet".to_owned(),
        Some(run) => match &run.outcome {
            Outcome::Ran { .. } => format!("ran {}", age(run.at)),
            Outcome::Failed { why } => format!("failed {}: {why}", age(run.at)),
        },
    };
    let paused = match &r.paused {
        None => "",
        Some(Paused::Owner) => " · paused",
        Some(Paused::ChatGone { .. }) => " · paused: its chat is gone",
    };
    format!("{} · {place} · {last}{paused}", r.when())
}

/// One routine's row and its edit.
#[component]
async fn row(r: Routine) -> Result<impl View> {
    let to = format!("/routines/{}", r.id);
    let (now, flip, to_pause) = if r.paused.is_some() {
        ("resume", "resumed", "false")
    } else {
        ("pause", "paused", "true")
    };
    let Trigger::Schedule(s) = &r.trigger;
    let when = String::from(s.clone());
    Ok(view! {
        <div class="acr" data-routine=(r.id.clone())>
            <span class="ptg">"⟳"</span>
            <span class="sk">(r.name.clone())</span>
            <span class="st m">(line(&r))</span>
            <form method="post" action=(to.clone())>
                <input type="hidden" name="paused" value=(to_pause)>
                <button
                    type="submit"
                    class="sec"
                    aria-label=(format!("{now} {}: it will be {flip}", r.name))
                >
                    (now)
                </button>
            </form>
            <form method="post" action=(format!("{to}/delete"))>
                <button
                    type="submit"
                    class="sec"
                    aria-label=(format!("delete {}", r.name))
                >
                    "delete"
                </button>
            </form>
        </div>
        <details class="thx">
            <summary>(format!("edit {}", r.name))</summary>
            <form method="post" action=(to)>
                <input
                    name="name"
                    required=(true)
                    aria-label="name"
                    value=(r.name.clone())
                >
                <input
                    name="when"
                    required=(true)
                    aria-label="schedule (cron)"
                    value=(when)
                >
                <textarea name="prompt" required=(true) aria-label="prompt">
                    (r.prompt.clone())
                </textarea>
                <button type="submit">"save"</button>
            </form>
        </details>
    })
}

#[component]
async fn routines(list: Vec<Routine>) -> Result<impl View> {
    Ok(view! {
        document(
            title: "routines",
            <main class="sh set mcp" aria-labelledby="sht-routines">
                <div class="hd pvh">
                    <a class="gl" href="/#settings" aria-label="back to settings">
                        "‹"
                    </a>
                    <h1 id="sht-routines">"routines"</h1>
                </div>
                <div class="top">
                    for r in list.clone() {
                        row(r: r)
                    }
                    <p class="ds">
                        "ask an agent in chat to make one: \"every Monday at 9, check the renewals\""
                    </p>
                </div>
            </main>
        )
    })
}
