# The callers, as they will read (#75)

Written before the types: each block is how the code that uses routines reads after the change.

## 1. An agent makes a routine in chat (q764, design `chat-first` §4)

The agent calls cooee's own MCP tool. The owner sees one `.rt` line in the chat, nothing to approve.

```json
{ "name": "create_routine", "arguments": {
    "name": "Monday renewals",
    "when": "0 9 * * 1",
    "prompt": "Which client renewals are due this month? Draft a short note for each." } }
```

```json
{ "name": "create_routine", "arguments": {
    "name": "CI went red",
    "github": { "repo": "jirathip-dev/cooee", "events": ["workflow_run.completed"],
                "match": { "workflow_run.conclusion": "failure" } },
    "place": "new_thread",
    "prompt": "CI failed on {{workflow_run.head_branch}} ({{workflow_run.html_url}}). Find why and say what to fix." } }
```

```rust
// cooee-server lib.rs: the tool's route, beside /mcp/agent/remember
#[route(POST "/mcp/agent/routine")]
async fn mcp_routine(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<cooee_core::Item>> {
    let token = bearer(cx);
    Ok(Answer(blocking(cx, move |hub| {
        let ask: routines::Ask = serde_json::from_value(a.server).map_err(refused)?;
        hub.routines().create((&a.thread, &token), ask, hub::now())   // stores it, then the `.rt` item
    }).await))
}
```

The tool's error text is the agent's to read: `BAD_SCHEDULE: "*/5 * * * *" runs more often than every 15 minutes`,
`NOT_HERE: a Workflow thread or a routine's own run can't make routines`, `NO_HOOKS: [hooks] isn't set up on this Mac`.

## 2. The scheduler (one thread in `cooee serve`, beside the models' daily refresh)

```rust
// cooee-server lib.rs, serve(): after Hub::open
let runner = Runner::open(Arc::clone(&hub), &data, config.hooks.clone())?;
let (clock, beat) = (Arc::clone(&runner), Arc::clone(&runner));
std::thread::spawn(move || loop {
    clock.tick(hub::now());                          // every routine whose due time passed: run it once
    std::thread::sleep(Duration::from_secs(30));
});
std::thread::spawn(move || loop {
    beat.catch_up(hub::now());                       // GitHub's delivery log: what the hook path missed
    std::thread::sleep(Duration::from_mins(15));
});
```

```rust
// cooee-server routines.rs
pub fn tick(&self, now: u64) {
    let offset = self.clock.offset();                // the Mac's UTC offset, `date +%z`, read hourly
    for (id, due) in self.store.due(now, offset) {   // Due only when > last_due: a restart re-reads last_due
        let key = RunKey::Due(id.clone(), due);      // `routine-<id>-<due>`: the hub's random_id
        let outcome = self.run(&id, &key, &Fields::due(due));
        self.store.ran(&id, Some(due), outcome);     // after the run: a crash between the two re-runs the
    }                                                // same key, which the hub adopts (no second thread)
}
```

## 3. The webhook (its own listener; the main Gate is untouched)

```rust
// cooee-server routines.rs: the only route on `[hooks] bind`
#[route(POST "/hooks/github")]
async fn github(cx: &Cx, body: Bytes) -> Result<Response> {
    let runner = app_context::<Arc<Runner>>(cx).clone();
    let headers = Headers::from(cx);                 // X-GitHub-Event, X-GitHub-Delivery, X-Hub-Signature-256
    let answer = tokio::task::spawn_blocking(move || {
        let secret = runner.secret()?;               // secret:GITHUB_WEBHOOK_SECRET, else 503 (GitHub retries; poll catches up)
        let d = Delivery::signed(&headers, &body, &secret)?;   // 400 bad headers, 401 bad signature, 413 > 2 MiB
        Ok(runner.deliver(d, hub::now()))            // 200 duplicate or pong, 202 {runs} accepted
    }).await?;
    reply(answer)
}
```

```rust
// cooee-server routines.rs: catch-up, same ingest, a different proof
pub fn catch_up(&self, now: u64) {
    for repo in self.store.repos() {                 // the repos any routine's github trigger names
        match github::missed(&self.token()?, &repo, &self.hooks.url, &self.store.seen(), now) {
            Ok(list) => list.into_iter().for_each(|d| { self.deliver(d, now); }),
            Err(e) => eprintln!("cooee: routines: catch-up {repo}: {e}"),
        }
    }
}
```

## 4. A run (schedule or event, one function)

```rust
fn run(&self, id: &str, key: &RunKey, fields: &Fields) -> Outcome {
    let r = self.store.get(id)?;
    let text = r.prompt.render(fields);              // {{a.b}} from the payload; {{due}}, {{event}}, {{repo}}
    let from = From::Routine(Mark { id: r.id.clone(), name: r.name.clone() });
    match &r.place {
        Place::Chat { thread } => self.hub.send_from(thread, &text, &key.random_id(), from)
            .map(|_| Outcome::Ran { thread: thread.clone() }),   // a working turn queues it, as the owner's would
        Place::New { who } => {
            let (agent, folder, recipe) = self.recipe(who)?;    // lib.rs's recipe(), unchanged
            self.hub.create_later((&agent, &who.agent), &folder, (&text, &key.random_id()),
                                  Recipe { routine: Some(from.mark()), ..recipe })
                .map(|thread| Outcome::Ran { thread })
        }
    }
    .unwrap_or_else(|e| Outcome::from_error(e))      // a closed or gone chat: Failed and the routine pauses
}
```

## 5. Settings › routines and the thread list

```text
GET  /routines                      page: each routine, its trigger in words, where it posts, last run, [pause] [edit] [delete]
GET  /routines.json                 Routine[] (the API)
POST /routines/{id}                 {name?, when?, prompt?, paused?}   → Routine
POST /routines/{id}/delete          {}                                 → null (a confirm first, as any deletion)
```

```ts
// web/ts list row: a thread a routine started, or whose last message a routine sent
row.routine && mark.textContent = `⟳ ${row.routine}`;
```

```rust
// cooee-core chat.rs Summary: the name kept on the thread (a new-thread run), else on its last `you` item
routine: meta.routine.as_ref().or(last_you.and_then(Item::routine)).map(|m| m.name.clone()).unwrap_or_default(),
```
