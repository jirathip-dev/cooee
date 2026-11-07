// Pairing (#47), where it needs script. On the pairing page: the six code boxes (a digit moves on, ⌫ goes back, a
// paste fills all six, the sixth sends it; a QR link's code sends itself), the "too many tries" countdown, offline,
// asking a paired device to approve (the check word, waiting, declined) and signing back in with a passkey. On the
// thread list: the passkey offer after pairing and the approve cards (a passkey with Face ID approves; a decline can be
// undone for 10 s). In Settings › devices: pair a device (its QR and code) and the passkeys. It talks to the server only
// through the JSON API in docs/api.md; the passkeys are WebAuthn (ES256, this device's platform authenticator).
import { $, make } from "./look.js";

type Assertion = { id: string; client: string; auth: string; sig: string };
type Asking = { id: string; word: string; label: string; at: number };
type Key = { id: string; label: string; created: number; used: number };

async function post<T = string>(url: string, body: unknown): Promise<T> {
  const r = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok)
    throw new Error(r.headers.has("cooee-error") ? await r.text() : `cooee couldn't do that (error ${r.status}).`);
  return r.headers.get("content-type")?.includes("json") ? r.json() : (r.text() as Promise<T>);
}

const b64 = (b: ArrayBuffer): string =>
  btoa(String.fromCharCode(...new Uint8Array(b)))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
const bytes = (s: string): ArrayBuffer =>
  Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0)).buffer;
const clock = (s: number): string => `${Math.floor(s / 60)}:${String(Math.max(0, s) % 60).padStart(2, "0")}`;
const canPasskey = "PublicKeyCredential" in window && isSecureContext;
const NONE = "No passkey for this cooee on this device. Pair with a code instead.";

/// A new passkey on this device's platform authenticator (Face ID, Touch ID), kept by the server.
async function savePasskey(): Promise<void> {
  const { challenge, ids } = await post<{ challenge: string; ids: string[] }>("/passkey/challenge", {});
  const c = (await navigator.credentials.create({
    publicKey: {
      challenge: bytes(challenge),
      rp: { name: "cooee", id: location.hostname },
      user: {
        id: crypto.getRandomValues(new Uint8Array(16)).buffer,
        name: `cooee · ${location.hostname}`,
        displayName: "cooee",
      },
      pubKeyCredParams: [{ type: "public-key", alg: -7 }],
      authenticatorSelection: {
        authenticatorAttachment: "platform",
        residentKey: "required",
        userVerification: "required",
      },
      attestation: "none",
      excludeCredentials: ids.map((id) => ({ type: "public-key", id: bytes(id) })),
    },
  })) as PublicKeyCredential | null;
  const r = c?.response as AuthenticatorAttestationResponse | undefined;
  const key = r?.getPublicKey();
  if (!c || !r || !key) throw new Error("This device made no passkey.");
  await post("/passkeys", {
    id: c.id,
    key: b64(key),
    client: b64(r.clientDataJSON),
    auth: b64(r.getAuthenticatorData()),
  });
}

/// A passkey's signature over a fresh challenge, after Face ID.
async function sign(): Promise<Assertion> {
  const { challenge } = await post<{ challenge: string }>("/passkey/challenge", {});
  const c = (await navigator.credentials
    .get({ publicKey: { challenge: bytes(challenge), rpId: location.hostname, userVerification: "required" } })
    .catch(() => null)) as PublicKeyCredential | null;
  if (!c) throw new Error(NONE);
  const r = c.response as AuthenticatorAssertionResponse;
  return { id: c.id, client: b64(r.clientDataJSON), auth: b64(r.authenticatorData), sig: b64(r.signature) };
}

/// `button`s in a row.
const acts = (...buttons: [string, () => void, string?][]): HTMLElement => {
  const row = make("div", "acts");
  for (const [text, go, cls] of buttons) {
    const b = make("button", cls ?? "", text) as HTMLButtonElement;
    b.type = "button";
    b.onclick = go;
    row.append(b);
  }
  return row;
};

// ---- The pairing page.
const form = $<HTMLFormElement>("#code");
if (form) {
  const boxes = [...form.querySelectorAll<HTMLInputElement>(".cb input")];
  const go = $<HTMLButtonElement>("button[type=submit]", form);
  const code = (): string => boxes.map((b) => b.value).join("");
  const ready = (): boolean => /^\d{6}$/.test(code());
  const sync = (): void => {
    if (go && !boxes[0]?.disabled) go.disabled = !ready();
  };
  const fill = (from: number, digits: string): void => {
    const ds = digits.replace(/\D/g, "").slice(0, 6 - from);
    ds.split("").forEach((d, i) => {
      const b = boxes[from + i];
      if (b) b.value = d;
    });
    for (const b of boxes) b.value = b.value.replace(/\D/g, "").slice(0, 1);
    boxes[Math.min(from + ds.length, 5)]?.focus();
    sync();
    if (ready() && ds) send();
  };
  boxes.forEach((b, i) => {
    b.addEventListener("input", () => fill(i, b.value));
    b.addEventListener("keydown", (e) => {
      if (e.key === "Backspace" && !b.value && i > 0) {
        e.preventDefault();
        const prev = boxes[i - 1];
        if (prev) {
          prev.value = "";
          prev.focus();
        }
        sync();
      }
    });
    b.addEventListener("paste", (e) => {
      e.preventDefault();
      fill(i, e.clipboardData?.getData("text") ?? "");
    });
  });
  sync();

  // Offline: the card says when it last tried; [try again] sends the code again.
  const offline = $("#offline");
  let tried = 0;
  let tick = 0;
  const unreachable = (again: () => void): void => {
    tried = Date.now();
    if (!offline) return;
    offline.hidden = false;
    const at = $("#tried", offline);
    clearInterval(tick);
    const show = (): void => {
      if (at) at.textContent = `tried ${clock(Math.round((Date.now() - tried) / 1000))} ago`;
    };
    show();
    tick = setInterval(show, 1000);
    const retry = $<HTMLButtonElement>("#retry", offline);
    if (retry)
      retry.onclick = () => {
        offline.hidden = true;
        clearInterval(tick);
        again();
      };
  };

  // The form posts itself; the server's answer (the list, or this page saying why) is opened.
  let sending = false;
  function send(): void {
    if (sending) return;
    sending = true;
    const body = new URLSearchParams(new FormData(form as HTMLFormElement) as unknown as Record<string, string>);
    fetch(form?.action ?? "/pair", { method: "POST", body })
      .then((r) => location.replace(r.url))
      .catch(() => {
        sending = false;
        unreachable(send);
      });
  }
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    if (ready()) send();
  });
  // A QR link's code: it sends itself.
  if (form.querySelector(".cb.ok") && ready()) send();

  // Too many tries: the boxes open again when the minute is up.
  const limit = $("#limit");
  if (limit) {
    let wait = Number(limit.dataset.wait) || 60;
    const left = $(".tm", limit);
    const t = setInterval(() => {
      wait -= 1;
      if (left) left.textContent = clock(wait);
      if (wait <= 0) {
        clearInterval(t);
        location.replace("/pair");
      }
    }, 1000);
  }

  // Ask to pair: the check word, waiting for a paired device's answer.
  const welcome = $("section.wel");
  const [wait, no] = [$("#ask-wait"), $("#ask-no")];
  let claim = "";
  let poll = 0;
  const back = (): void => {
    clearInterval(poll);
    if (wait) wait.hidden = true;
    if (welcome) welcome.hidden = false;
  };
  const refused = (by: string, why: string): void => {
    back();
    if (welcome) welcome.hidden = true;
    const [b, w] = [$("#ask-by"), $("#ask-why")];
    if (b) b.textContent = by;
    if (w) w.textContent = why;
    if (no) no.hidden = false;
  };
  async function ask(): Promise<void> {
    if (no) no.hidden = true;
    let r: { word: string; claim: string };
    try {
      r = await post("/pair/request", {});
    } catch {
      return unreachable(ask);
    }
    claim = r.claim;
    for (const id of ["#ask-word", "#ask-word2"]) {
      const e = $(id);
      if (e) e.textContent = r.word;
    }
    if (welcome) welcome.hidden = true;
    if (wait) wait.hidden = false;
    const until = Date.now() + 5 * 60 * 1000;
    const left = $("#ask-left");
    const tick = (): void => {
      if (left) left.textContent = `expires in ${clock(Math.round((until - Date.now()) / 1000))}`;
    };
    tick();
    clearInterval(poll);
    poll = setInterval(async () => {
      tick();
      const c = await fetch("/pair/claim", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ claim }),
      }).catch(() => null);
      if (c?.status === 200) {
        clearInterval(poll);
        location.replace("/?paired");
      } else if (c?.status === 403) {
        refused(await c.text(), `Your request to pair (${r.word}) was declined. If that was a mistake, ask again.`);
      } else if (c?.status === 410) refused("Your request ran out.", "Nobody approved it in 5 minutes. Ask again.");
    }, 1000);
  }
  const askGo = $<HTMLButtonElement>("#ask-go");
  if (askGo) askGo.onclick = ask;
  const again = $<HTMLButtonElement>("#ask-again");
  if (again) again.onclick = ask;
  const cancel = $<HTMLButtonElement>("#ask-cancel");
  if (cancel)
    cancel.onclick = () => {
      post("/pair/cancel", { claim }).catch(() => {});
      back();
    };

  // Signing back in with a passkey.
  const pk = $<HTMLButtonElement>("#pk-go");
  const pkErr = $("#pk-err");
  if (pk && !canPasskey) pk.closest(".lrw")?.remove();
  if (pk)
    pk.onclick = async () => {
      if (pkErr) pkErr.hidden = true;
      try {
        await post("/passkey/signin", await sign());
        location.replace("/");
      } catch (x) {
        const et = pkErr && $(".et", pkErr);
        if (et) et.textContent = x instanceof Error ? x.message : NONE;
        if (pkErr) pkErr.hidden = false;
      }
    };

  // The connection dot.
  const dot = $("#cdot");
  const online = (): void => {
    dot?.classList.toggle("off", !navigator.onLine);
    dot?.setAttribute("aria-label", navigator.onLine ? "connected" : "offline");
  };
  addEventListener("online", online);
  addEventListener("offline", online);
  online();
}

// ---- The thread list: the passkey offer after pairing, and the approve cards.
const list = location.pathname === "/" ? $("main.list") : null;
if (list) {
  const top = make("div", "pq");
  top.id = "pairing";
  list.prepend(top);
  const me = /iPhone|iPad/.test(navigator.userAgent)
    ? "Face ID"
    : /Macintosh/.test(navigator.userAgent)
      ? "Touch ID"
      : "passkey";

  if (new URLSearchParams(location.search).has("paired")) {
    history.replaceState(null, "", "/");
    if (canPasskey) offer();
  }
  function offer(): void {
    const card = make("div", "pc");
    card.dataset.k = "g";
    card.setAttribute("role", "group");
    card.setAttribute("aria-label", "save a passkey");
    const q = make("p");
    q.append(
      make("b", "", "Save a passkey?"),
      ` ${me === "passkey" ? "A passkey" : me} then signs this device back in if the browser forgets it, and approves new devices.`,
    );
    card.append(
      make("p", "et", "✓ paired"),
      q,
      acts(
        [
          "save a passkey",
          async () => {
            try {
              await savePasskey();
              card.replaceChildren(make("p", "et", "✓ passkey saved"), make("p", "m", "settings › devices › passkeys"));
            } catch (x) {
              card.append(make("p", "err", x instanceof Error ? x.message : String(x)));
            }
          },
        ],
        ["not now", () => card.remove(), "q"],
      ),
      make("p", "m", "later: settings › devices › passkeys"),
    );
    top.prepend(card);
  }

  // Requests to pair: a card each while it waits; one answered elsewhere or run out goes by itself.
  const cards = new Map<string, HTMLElement>();
  let keys: boolean | null = null;
  async function asks(): Promise<void> {
    if (document.hidden) return;
    const r = await fetch("/pair/asks").catch(() => null);
    if (!r?.ok) return;
    const open: Asking[] = await r.json();
    for (const [id, card] of cards)
      if (!open.some((a) => a.id === id) && !card.dataset.done) {
        card.remove();
        cards.delete(id);
      }
    if (open.length && keys === null) {
      const k = await fetch("/passkeys").catch(() => null);
      keys = k?.ok ? ((await k.json()) as Key[]).length > 0 : null;
    }
    for (const a of open) if (!cards.has(a.id)) cards.set(a.id, card(a));
  }
  function card(a: Asking): HTMLElement {
    const c = make("div", "apc");
    c.setAttribute("role", "group");
    c.setAttribute("aria-label", `${a.label} asks to pair: ${a.word}`);
    const head = make("p", "aph");
    head.append(make("b", "", "Pair a new device?"), make("span", "m", ago(a.at)));
    const word = make("p", "cwl");
    word.append(make("span", "m", "check word"), make("b", "", a.word));
    const err = make("p", "err");
    const decline = (): void => {
      c.dataset.done = "1";
      let undone = false;
      const timer = setTimeout(() => {
        if (!undone) post("/pair/answer", { id: a.id, yes: false }).catch(() => {});
        c.remove();
      }, 10000);
      c.className = "pc";
      const sees = make("p", "", "It won't be paired, and it sees that you said no.");
      fetch("/devices")
        .then((r) => (r.ok ? r.json() : []))
        .then((ds: { label: string; me: boolean }[]) => {
          const mine = ds.find((d) => d.me);
          if (mine) sees.textContent = `It won't be paired, and it sees “${mine.label} said no”.`;
        })
        .catch(() => {});
      const row = acts([
        "undo",
        () => {
          undone = true;
          clearTimeout(timer);
          delete c.dataset.done;
          c.replaceWith(card(a));
          c.remove();
        },
        "q",
      ]);
      row.append(make("span", "m", "for 10 s"));
      c.replaceChildren(make("p", "et", `Declined · ${a.label}`), sees, row);
    };
    const approve = async (): Promise<void> => {
      err.textContent = "";
      try {
        const passkey = await sign();
        await post("/pair/answer", { id: a.id, yes: true, passkey });
      } catch (x) {
        err.textContent = x instanceof Error ? x.message : String(x);
        return;
      }
      c.dataset.done = "1";
      c.className = "pc";
      c.dataset.k = "g";
      const title = make("p", "et", `✓ ${a.label} paired`);
      const rename = (): void => {
        const box = make("div", "oth");
        const name = make("input") as HTMLInputElement;
        name.value = a.label;
        name.maxLength = 40;
        name.setAttribute("aria-label", "its name");
        const save = make("button", "", "save") as HTMLButtonElement;
        save.type = "button";
        save.onclick = async () => {
          err.textContent = "";
          try {
            title.textContent = `✓ ${await post("/devices/rename", { id: a.id, label: name.value })} paired`;
            box.replaceWith(row);
          } catch (x) {
            err.textContent = x instanceof Error ? x.message : String(x);
          }
        };
        box.append(name, save);
        row.replaceWith(box);
        name.focus();
      };
      const row = acts(["rename it", rename], ["done", () => c.remove()]);
      c.replaceChildren(
        title,
        make("p", "", "It's in settings › devices. The other devices' cards cleared."),
        row,
        err,
      );
    };
    c.append(head, make("p", "who", `${a.label}, over your tailnet`), word);
    if (keys === false || !canPasskey) {
      c.append(make("p", "m", "Approving needs a passkey. Save one on this device, or approve on another one."));
      if (canPasskey)
        c.append(
          acts(
            [
              "save a passkey",
              async () => {
                try {
                  await savePasskey();
                  keys = true;
                  c.replaceWith(card(a));
                } catch (x) {
                  err.textContent = x instanceof Error ? x.message : String(x);
                }
              },
            ],
            ["decline", decline, "q"],
          ),
        );
      else c.append(acts(["decline", decline, "q"]));
    } else c.append(acts([`approve with ${me}`, approve], ["decline", decline, "q"]));
    c.append(err);
    top.append(c);
    cards.set(a.id, c);
    return c;
  }
  const ago = (at: number): string => {
    const m = Math.round((Date.now() - at) / 60000);
    return m < 1 ? "just now" : `${m}m ago`;
  };
  asks();
  setInterval(asks, 3000);
  addEventListener("visibilitychange", asks);
}

// ---- Settings › devices: pair a device (its QR and code), and the passkeys.
addEventListener("cooee-sheet", (e) => {
  if ((e as CustomEvent).detail !== "settings") return;
  pairBox();
  passkeys();
});
function pairBox(): void {
  const box = $("#s-pair-box");
  if (!box) return;
  const open = make("button", "", "pair a device") as HTMLButtonElement;
  open.type = "button";
  open.onclick = async () => {
    let r: { code: string; link: string; svg: string; host: string };
    try {
      r = await post("/devices/pair", {});
    } catch (x) {
      return box.replaceChildren(open, make("p", "err", x instanceof Error ? x.message : String(x)));
    }
    const until = Date.now() + 10 * 60 * 1000;
    const tile = make("span", "qrt");
    const img = make("img") as HTMLImageElement;
    img.alt = `QR code: the link to pair with code ${r.code}`;
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(r.svg)}`;
    tile.append(img);
    const left = make("p", "m ctr");
    const show = (): void => {
      left.textContent = `one use · expires in ${clock(Math.round((until - Date.now()) / 1000))} · ${r.host}`;
    };
    show();
    const t = setInterval(() => (box.isConnected ? show() : clearInterval(t)), 1000);
    const panel = make("div", "pc");
    panel.append(
      tile,
      make("p", "ctr", "Scan it with the new device's Camera, or type the code there."),
      make("p", "bc", `${r.code.slice(0, 3)} ${r.code.slice(3)}`),
      left,
      acts(
        [
          "done",
          () => {
            clearInterval(t);
            box.replaceChildren(open);
          },
          "q",
        ],
        [
          "new code",
          () => {
            clearInterval(t);
            open.click();
          },
        ],
      ),
    );
    box.replaceChildren(panel);
  };
  box.replaceChildren(open);
}
async function passkeys(): Promise<void> {
  const box = $("#s-passkeys-box");
  if (!box) return;
  const r = await fetch("/passkeys").catch(() => null);
  if (!r?.ok) return box.replaceChildren(make("p", "ds", "passkeys show when online"));
  const day = (at: number): string => new Date(at).toLocaleDateString(undefined, { day: "numeric", month: "short" });
  const keys: Key[] = await r.json();
  const rows = keys.map((k) => {
    const row = make("div", "lrw pkr");
    const what = make("span", "sk", k.label);
    what.append(make("small", "", ` · made ${day(k.created)}${k.used ? ` · used ${day(k.used)}` : ""}`));
    const b = make("button", "cn", "remove") as HTMLButtonElement;
    b.type = "button";
    b.setAttribute("aria-label", `remove the passkey made on ${k.label}`);
    b.onclick = async () => {
      if (!confirm(`Remove the passkey made on ${k.label}? It can't sign in or approve any more.`)) return;
      await post("/passkeys/remove", { id: k.id }).catch(() => {});
      passkeys();
    };
    row.append(what, b);
    return row;
  });
  if (!keys.length) rows.push(make("p", "ds", "none yet: a passkey signs a device back in and approves new ones"));
  if (canPasskey)
    rows.push(
      acts([
        "save a passkey here",
        () =>
          savePasskey().then(passkeys, (x) => box.append(make("p", "err", x instanceof Error ? x.message : String(x)))),
        "cn",
      ]),
    );
  box.replaceChildren(...rows);
}
