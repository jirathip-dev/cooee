# Callers: how the code that uses it reads (#714)

Sketches, written before the design. Names are final; bodies are not. The shapes are in `shapes.md`.

## 1. A desktop client, from nothing to a streaming reply

The egui app is its own issue. This is what the server must make possible: the client holds an address, a cookie
jar and blocks' `Mirror`, and has no code that knows what a chat is.

```rust
let http = Http::new(address);                 // typed or pasted by the owner; the cookie jar is the whole session
let mut mirror = Mirror::new(2);

stage.set(&http.get("/screens/pair")?);        // open to anyone: a code field, "ask to pair", "sign in with this device"

// the owner types the six digits `cooee pair` printed on the server
let a: Answer = http.post("/screens/pair/action", &Action::new("/screens/pair", "code").value("code", digits))?;
// a.open is "/screens/sessions"; the jar now holds the device cookie

let (hello, frames) = http.stream("/stream")?; // one stream for every screen this device shows
mirror.hello(&hello)?;
mirror.open("/screens/sessions");
let sent = mirror.have();
mirror.synced(&sent, http.post(&format!("/stream/{}", hello.sid), &Sync { open: sent.clone(), close: vec![] })?);

// a row's tap says open "/screens/chat/<id>": open that key, sync once more, then only this loop
for f in frames {
    match mirror.frame(f) {
        Took::Patched | Took::Replaced => stage.set(mirror.screen(key)),
        Took::Gap => resync(&mut mirror, &http),               // the same one POST
        Took::Old => {}
    }
}

// the composer's send: one door for every action of a paired device
http.post("/screens/action", &Action::new(key, "send").block("say").values(text_files_and_nonce))?;
// the owner's message and the reply arrive as frames; the answer carries only a toast when it was refused
```

A lost connection, a restarted server and a closed laptop are one case: `hello` again, one sync with the revs
held, and each key answers nothing, the missed ops or the whole screen.

## 2. A screen module on the server

A screen is a function from cooee's state to a `Block`. It never writes an op, a frame or a rev.

```rust
// screens/chat.rs
#[route(GET "/screens/chat/{id}")]
pub async fn chat(cx: &Cx) -> Result<Answer<Block>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| screen(hub, &id, 0)).await))
}

pub(super) fn source(path: &str, who: &Who) -> Option<Source> {
    let id = path.strip_prefix("/screens/chat/")?.to_owned();
    matches!(who, Who::Device(_)).then(|| Source {
        wake: Wake::Thread(id.clone()),                        // rebuilt when this thread's hub events fire
        build: Box::new({ let id = id.clone(); move |hub, page| screen(hub, &id, page) }),
        seen: Some(Box::new(move |hub| read_to_newest(hub, &id))),
    })
}

pub(super) async fn act(cx: &Cx, a: &Action) -> Option<Result<blocks::Answer, HubError>> {
    let id = a.screen.strip_prefix("/screens/chat/")?;
    Some(match a.action.as_str() {
        "send" => turns::send(cx, id, Sent::read(&a.values)?).await.map(|_| blocks::Answer::default()),
        "stop" => blocking(cx, |hub| hub.cancel(id)).await.map(|()| blocks::Answer::default()),
        "older" => Ok(live(cx).more(&a.screen)),
        ...
    })
}

fn screen(hub: &Hub, id: &str, page: usize) -> Result<Block, HubError> {
    let (meta, _) = hub.thread(id)?;
    let slice = hub.slice(id, None, WINDOW * (page + 1), 0)?;
    let blocks = vec![older(&slice), log(&meta, &slice), state(&slice.turn), composer(&meta, &slice)];
    Ok(core(Screen::new().id("chat").title(meta.title).back("/screens/sessions").blocks(blocks)))
}

fn reply(group: &Group, turn: &Turn) -> Block {                // an agent's run of items, and the text still coming
    let mut blocks: Vec<Block> = group.items.iter().filter_map(said).collect();
    if let Some((markdown, writing)) = group.text(turn) {
        let text = Prose::new(Prose::read(markdown)).id(format!("{}-text", group.id));
        blocks.push(Block::Prose(Box::new(if writing { text.streaming(true) } else { text })));
    }
    Block::Message(Box::new(Message::new(Role::Agent, &group.name).id(&group.id).blocks(blocks)))
}
```

`routes/threads.rs` and `screens/chat.rs` both call `turns::send`, so a message sent through either door is kept,
restarted and stored once by the same code.

## 3. The publisher: one task per key somebody holds

```rust
// live.rs
loop {
    wake.next().await;                          // the thread's events, the home list, or a 1 s tick (the pair screen)
    sleep_until(last + pace).await;             // pace: 50 ms, or ten times what the last build and put took
    let block = (source.build)(&hub, page)?;    // on a blocking thread
    if let Put::Changed(d) = shelf.put(&(), &key.path, &block)? {
        for stream in &holders { stream.push(Arc::clone(&d.frame)) }
        if let Some(seen) = &source.seen { seen(&hub) }   // a chat marks its thread read up to its newest item
    }
}
```

The task ends when the last holder closes the key, so a thread nobody looks at costs nothing, and its pushes go
out again (the hub already asks "does anyone listen?").

## 4. One turn on the wire

The owner sends "check the tests"; the agent writes, calls a tool, writes on, ends. Key `/screens/chat/k3`, the
thread's last item was 6.

```json
{"k":"/screens/chat/k3","from":11,"rev":12,"ops":[{"op":"insert","into":"log","block":{"type":"message","id":"m7","role":"owner","name":"You","blocks":[{"type":"prose","id":"i7","parts":[{"part":"para","spans":[{"text":"check the tests"}]}]}]}},{"op":"merge","id":"say","fields":{"working":{"since":"2026-11-21T09:00:00Z"}}}]}
{"k":"/screens/chat/k3","from":12,"rev":13,"ops":[{"op":"insert","into":"log","block":{"type":"message","id":"m8","role":"agent","name":"Scout","blocks":[{"type":"prose","id":"m8-text","parts":[{"part":"para","spans":[{"text":"I will run"}]}],"streaming":true}]}}]}
{"k":"/screens/chat/k3","from":13,"rev":14,"ops":[{"op":"append","id":"m8-text","text":" them now."}]}
{"k":"/screens/chat/k3","from":14,"rev":15,"ops":[{"op":"insert","into":"m8","before":"m8-text","block":{"type":"tool","id":"i8","name":"run","state":"running","summary":"cargo test"}}]}
{"k":"/screens/chat/k3","from":15,"rev":16,"ops":[{"op":"merge","id":"i8","fields":{"state":"done"}},{"op":"append","id":"m8-text","text":" All 212 pass."}]}
{"k":"/screens/chat/k3","from":16,"rev":17,"ops":[{"op":"merge","id":"m8-text","fields":{"streaming":null}},{"op":"merge","id":"say","fields":{"working":null}}]}
```

The agent's message is `m8` before item 8 exists, because 8 is the id its first item will get. When the reply is
stored (item 9) its text stays the block `m8-text`, so the last frame only clears `streaming`.

## 5. The tests

Each drives the real server from `crates/cooee/tests`, with blocks' `Mirror` as the client.

```rust
#[test]
fn a_reply_streams_into_its_message_by_appends_and_a_reconnect_resumes_it() {
    let (s, t) = server_with_a_thread();
    let key = format!("/screens/chat/{t}");
    let mut m = s.follow(&[&key]);                              // GET /stream, hello, one sync
    s.agent.says("Reading ");
    s.agent.says("both files.");
    let frames = m.until(|screen| text_of(screen, "m2-text") == "Reading both files.");
    assert!(frames.iter().all(|f| !merges_parts(f)));           // the text came by `append`, never a merge of `parts`
    m.cut();                                                    // the connection drops mid-reply
    s.agent.says(" Done.");
    s.agent.replies();
    m.reconnect();                                              // hello, one sync with the rev held
    assert_eq!(m.screen(&key), s.get(&key));                    // the mirror holds what GET answers
    assert_eq!(id_of_the_reply_text(m.screen(&key)), "m2-text");  // the block that streamed is the stored one
}
```

The others, by name:

- `a_stream_survives_dropped_doubled_and_reordered_frames`
- `a_send_with_the_same_nonce_twice_is_one_item_and_one_prompt_through_either_door`
- `a_tool_call_is_one_tool_block_updated_in_place`
- `a_sent_file_is_an_attachment_that_opens`
- `a_typed_code_pairs_and_five_wrong_codes_lock_the_form_and_post_pair_together`
- `an_asking_device_sees_its_word_then_approved_and_gets_its_cookie_only_when_it_collects`
- `a_stream_without_a_device_opens_only_the_pair_screen`
- `a_declined_or_expired_request_never_gets_a_cookie`
- `a_device_key_signs_in_and_a_wrong_signature_does_not`
