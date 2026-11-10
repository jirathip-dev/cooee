//! Who a chat runs as, said with the provider's name and never the agent's id (#48), and Settings › providers, the
//! flat list of accounts.

use cooee_core::{Account, Sheet, Status, Usage, Window};
use topcoat::{
    Result,
    view::{View, component, view},
};

/// A provider's name for ACP agent `agent` (`claude` for `claude-acp`): never the agent's id on screen.
#[must_use]
pub fn named(agent: &str, providers: &[Sheet]) -> String {
    providers
        .iter()
        .find(|s| s.provider.agent == agent)
        .map_or_else(
            || agent.strip_suffix("-acp").unwrap_or(agent).to_owned(),
            |s| s.provider.name.clone(),
        )
}

/// Who thread or pick `account` of `agent` runs as (an email, an account's name or `main login`); empty for an
/// agent no provider signs in.
#[must_use]
pub fn who(agent: &str, account: &str, providers: &[Sheet]) -> String {
    let Some(sheet) = providers.iter().find(|s| s.provider.agent == agent) else {
        return String::new();
    };
    let found = sheet.accounts.iter().find(|(a, _)| a.name == account);
    match found {
        Some((a, s)) => a.shown(s),
        None if account.is_empty() => "main login".to_owned(),
        None => account.to_owned(),
    }
}

/// `<provider> · <account>`, or the provider alone.
pub(crate) fn said(name: &str, who: &str) -> String {
    if who.is_empty() {
        name.to_owned()
    } else {
        format!("{name} · {who}")
    }
}

/// An empty picker's row instead: why, and the line that fixes it with its copy button (`chat.ts`).
#[component]
pub async fn hint(why: &str, fix: &str, then: &str) -> Result<impl View> {
    Ok(view! {
        <p class="hn">
            (why)
            <code>(fix)</code>
            (then)
            <button type="button" data-copy=(fix) aria-label=(format!("copy {fix}"))>
                "copy"
            </button>
        </p>
    })
}

/// A provider's sign-in terminal (drawn by `chat.ts`), opened under the row that started it.
#[component]
pub async fn login_panel(provider: String, command: String) -> Result<impl View> {
    Ok(view! {
        <div class="lg" data-provider=(provider.clone()) hidden=(true)>
            <p class="ds">(format!("{command} · shed terminal"))</p>
            <pre class="out"></pre>
            <div class="lgf">
                <input
                    aria-label=(format!("the code from the {provider} sign-in page"))
                    placeholder="paste the code"
                    autocomplete="off"
                >
                <button type="button" class="lgs">"send"</button>
                <button type="button" class="sec cn">"cancel"</button>
            </div>
        </div>
    })
}

/// A signed-in account's usage in Settings › providers (#183): `5h — · 7d 40% · read 5m`, a window past its reset
/// at 0 %.
fn usage_line(u: Usage) -> String {
    let u = u.at(crate::now() / 1000);
    let pct = |w: Option<Window>| w.map_or("—".to_owned(), |w| format!("{}%", w.used_pct));
    let read = crate::age(u.read_at * 1000);
    format!(
        "5h {} · 7d {} · read {read}",
        pct(u.five_hour),
        pct(u.seven_day)
    )
}

/// One account in Settings › providers: `[provider] <who> ✓ signed in [sign out]`, `signed out [sign in]`, or why
/// it can't tell.
#[component]
async fn account_row(provider: String, a: Account, s: Status) -> Result<impl View> {
    Ok(view! {
        <div class="acr" data-provider=(provider.clone()) data-account=(a.name.clone())>
            <span class="ptg">(provider.clone())</span>
            <span class="sk">(a.shown(&s))</span>
            if s.unknown {
                // #106: can't tell (a status check timed out, or none ran yet): no sign-in offered as the fix.
                <span class="st m un">(s.line.clone())</span>
                if a.dir.is_some() {
                    <button type="button" class="sec sgo">"sign out"</button>
                }
            } else if s.signed_in {
                <span class="st g">"✓ signed in"</span>
                if let Some(u) = s.usage {
                    <span class="st m">(usage_line(u))</span>
                }
                <button type="button" class="sec sgo">"sign out"</button>
            } else {
                <span class="st m">"signed out"</span>
                if a.dir.is_some() {
                    <button type="button" class="sec sgo">"sign out"</button>
                }
                <button
                    type="button"
                    class="si"
                    data-provider=(provider)
                    data-account=(a.name.clone())
                >
                    "sign in"
                </button>
            }
        </div>
    })
}

/// Settings › providers (#48): one row per account, `[provider] <who> ✓ signed in [sign out]` or `signed out [sign
/// in]`; `[+ account]` per installed provider; one row per provider that isn't installed, `[copy install]`; then the
/// help link. A sign-in's terminal opens under its row.
#[component]
pub async fn providers_sheet(providers: Vec<Sheet>) -> Result<impl View> {
    let (out, mut installed): (Vec<Sheet>, Vec<Sheet>) = providers
        .into_iter()
        .partition(|s| s.accounts.iter().any(|(_, st)| st.install.is_some()));
    installed.retain(|s| !s.accounts.is_empty());
    Ok(view! {
        <div
            class="sh set"
            id="sh-providers"
            popover="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby="sht-providers"
        >
            <div class="hd pvh">
                <button
                    type="button"
                    class="gl"
                    data-open="settings"
                    aria-label="back to settings"
                >
                    "‹"
                </button>
                <b id="sht-providers">"providers"</b>
                <button
                    type="button"
                    class="gl"
                    data-close=(true)
                    popovertarget="sh-providers"
                    popovertargetaction="hide"
                    aria-label="close"
                >
                    "×"
                </button>
            </div>
            <div class="top">
                for sheet in installed {
                    for (a, s) in sheet.accounts.clone() {
                        account_row(provider: sheet.provider.name.clone(), a: a, s: s)
                    }
                    if !sheet.provider.home.is_empty() {
                        <div
                            class="acr add"
                            data-provider=(sheet.provider.name.clone())
                        >
                            <span class="ptg">(sheet.provider.name.clone())</span>
                            <span class="sk"></span>
                            <button type="button" class="sec ad">"+ account"</button>
                        </div>
                    }
                    // How its last sign-in ended, when it didn't simply finish.
                    if let Some(why) = sheet
                        .why
                        .clone()
                        .filter(|w| w != "the login command finished") {
                        <p class="ds wy">(why)</p>
                    }
                    login_panel(
                        provider: sheet.provider.name.clone(),
                        command: sheet.provider.login.join(" ")
                    )
                }
                for sheet in out {
                    <div class="acr ni">
                        <span class="ptg">(sheet.provider.name.clone())</span>
                        <span class="sk">"not installed"</span>
                        if !sheet.provider.install.is_empty() {
                            <button
                                type="button"
                                class="sec"
                                data-copy=(sheet.provider.install.clone())
                                title=(sheet.provider.install.clone())
                            >
                                "copy install"
                            </button>
                        }
                    </div>
                }
                <p class="ds">
                    <a
                        href="https://github.com/jirathip-dev/cooee/blob/main/docs/logins.md"
                        target="_blank"
                        rel="noopener"
                    >
                        "Where logins live"
                    </a>
                    " · on a headless Mac, keep the login Keychain unlocked"
                </p>
            </div>
        </div>
    })
}
