# Callers: how the code that uses it reads (#776)

Sketches, written before the design. Names are final; bodies are not. The shapes are in `shapes.md`.

## 1. The owner gives a persona a catalog

One key in `agent.toml`, set by the picker on the persona screen (none, `show`, `ask`). No key: text only, as today.

```toml
name = "Travel"
agent = "…"
blocks = "ask"
```

## 2. A session starts: the prompt

`Agent::brief` is the persona's session prompt today. It gains one part, and only when the key is set.

```rust
// agents.rs, in brief(): one more part after the skills
let blocks = self.blocks.as_deref().and_then(show::catalog).map(show::brief).unwrap_or_default();

// show/brief.rs
pub fn brief(catalog: &Catalog) -> String {
    format!("{LEAD}\n\n{}", model::prompt(catalog))      // blocks' text, whole and unchanged
}
```

`LEAD` is cooee's three sentences: answer with blocks by calling `show`, whose arguments are the JSON object the
text below calls your reply; if it refuses, fix what it lists and call it once more; an answer to your blocks comes
as your next message.

## 3. The persona calls `show`

The harness's model writes the reply as the tool's arguments. What it gets back is one of four things.

```json
{ "name": "show", "arguments": { "blocks": [
    { "type": "prose", "markdown": "Which plan?" },
    { "type": "choice", "action": "answer", "options": [
        { "value": "pro", "label": "Pro" }, { "value": "team", "label": "Team" } ] } ] } }
```

| The reply | The tool answers | The chat |
|---|---|---|
| reads | `Shown.` | the blocks, in the agent's message |
| refused, the turn's first refusal | `no.say()`, as an error | nothing yet |
| refused again | `Shown as plain text. Do not send it again.` | `no.plain()`: one prose block |
| any call after that in the turn | `NO_RETRY: …`, as an error; not read | nothing |

## 4. The hub: the whole loop

```rust
// hub/show.rs
pub fn show(&self, (id, token): (&str, &str), reply: &Value) -> Result<String, HubError> {
    let meta = self.token_of(id, token)?;
    let catalog = self.catalog_of(&meta)?;               // NO_CATALOG; NOT_HERE in a room
    let live = self.live(id)?;
    let mut l = lock(&live);
    let (shown, changes) = l.chat.show(catalog, &reply.to_string(), now());
    l.publish(self, changes)?;
    match shown {
        Shown::Drawn => Ok(DRAWN.to_owned()),
        Shown::Again(say) => Err(HubError::Refused(say)),
        Shown::Plain => Ok(PLAIN.to_owned()),
        Shown::Spent => Err(HubError::Refused(SPENT.to_owned())),
    }
}
```

```rust
// cooee-core chat/show.rs: the state is the chat's, beside the turn's start
pub fn show(&mut self, catalog: &Catalog, text: &str, at: u64) -> (Shown, Vec<Change>) {
    let next = self.items.last().map_or(1, |i| i.id + 1);
    match (std::mem::take(&mut self.tries), show::read(catalog, next, text)) {
        (Tries::Spent, _) => { self.tries = Tries::Spent; (Shown::Spent, vec![]) }
        (was, Ok(blocks)) => { self.tries = was.after_read(); self.keep(Shown::Drawn, blocks, at) }
        (Tries::Fresh, Err(no)) => { let say = no.say(); self.tries = Tries::Asked(no); (Shown::Again(say), vec![]) }
        (Tries::Asked(_), Err(no)) => { self.tries = Tries::Spent; self.keep(Shown::Plain, show::plain(&no, next), at) }
    }
}
```

A turn that ends while `Asked` (the persona did not call again) stores the first refusal's `plain`, in
`Chat::end`, so what the persona wrote for the owner is never lost.

## 5. The chat draws it

```rust
// screens/chat.rs, where a turn's items become its message's blocks
Body::Blocks { blocks, sent } => g.blocks.extend(drawn(item.id, blocks, sent.as_ref())),
```

`drawn` is the stored blocks as they are, then one footnote line (`✓ Pro · iPhone`) once the reply is answered.
The JSON routes send the same item: `{"kind": "blocks", "blocks": […], "sent": {…}}`.

## 6. An answer comes back

A model-written block's id is `b<item>.<n>`, and no id of cooee's own holds a `.`, so the chat's door tells them
apart by the block, before it looks at the action's name.

```rust
// screens/chat.rs, first line of on()
if let Some(item) = show::item_of(a) {
    let (t, by, a) = (Again::of(cx, id), paired(cx).label, a.clone());
    return turns::off(move || t.act(item, &a, &by)).await;      // restarts a gone agent, as a send does
}
```

```rust
// hub/show.rs
pub fn act(&self, id: &str, item: u64, a: &Action, by: &str) -> Result<(), HubError> {
    let live = self.live(id)?;
    let told = {
        let mut l = lock(&live);
        let catalog = self.catalog_of(&l.meta)?;
        let (told, changes) = l.chat.act(catalog, item, a, by, now()).map_err(refused)?;
        l.publish(self, changes)?;                               // the reply now shows its line
        told
    };
    match told {
        Some(t) => self.enqueue(id, &t.random_id, |_| Ok(t.queued())).map(drop),
        None => Ok(()),                                          // the same answer again: nothing to do
    }
}
```

What the persona gets as its next turn, and nothing else:

```
[The owner answered your blocks]: answer
{"choice":"pro"}
```

## 7. The tests, as they read

```rust
#[test]
fn a_choice_is_drawn_and_its_pick_is_the_personas_next_turn() {
    let cooee = with_persona("travel", r#"blocks = "ask""#);
    let id = cooee.start("travel", &format!("show {CHOICE}"));           // the fake agent calls the tool
    let screen = cooee.json(&format!("/screens/chat/{id}"));
    assert_eq!(block(&screen, "b3.1")["type"], "choice");
    cooee.act(&id, "b3.1", "answer", json!({ "choice": "pro" }));
    assert!(cooee.next_prompt(&id).ends_with("answer\n{\"choice\":\"pro\"}"));
}

#[test]
fn a_reply_refused_twice_is_plain_after_one_retry_and_a_third_is_not_read() {
    let mut chat = Chat::default();
    let catalog = show::catalog("show").unwrap();
    let outcomes = [BAD, BAD, GOOD].map(|text| chat.show(catalog, text, 1).0);
    assert!(matches!(outcomes, [Shown::Again(_), Shown::Plain, Shown::Spent]));
    assert_eq!(kinds(&chat), ["prose"]);
}
```

The others: a button in a `show` reply and a `wipe` action in an `ask` reply are each refused and store nothing;
`brief` holds `model::prompt(catalog)` whole; `{"choice": "enterprise"}` answers 400 and the fake agent gets no
prompt; a persona with no key lists no `show` tool and its brief is byte for byte today's.

## 8. The eval run

```sh
# in a scratch cooee (its own config and data), signed in through the host's shed
cooee send --new eval-a "$prompt" --wait --json | jq -r .reply            # what scripts/blocks-eval-model.sh does

# in a scratch clone of blocks at the pin, once per model
blocks eval fixtures/evals --replies fixtures/evals/replies/<model> \
    --model "$COOEE/scripts/blocks-eval-model.sh eval-a"
blocks eval fixtures/evals --replies fixtures/evals/replies/<model> > fixtures/evals/replies/<model>.txt
```
