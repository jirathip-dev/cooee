use serde_json::{Value, json};

use crate::mcp::Own;

fn shape() -> Value {
    json!({
        "name": "create_routine",
        "description": "Make a routine (when the owner asks for something on a schedule, on a GitHub event or on a signed POST from elsewhere): at each due time (when), matching event (github) or signed POST (webhook) its prompt is sent to you, in this chat (place chat, the default) or as a new thread like this one (place new_thread). The owner sees it in this chat and changes it in Settings › routines; a github or webhook one is saved only once the owner approves its card; a webhook's POST body follows the prompt as untrusted data; a webhook's silence rule (silence: N minutes) alerts the owner once, with no agent run, when no signed POST arrives for N minutes, and says once when they're back; a signed POST whose JSON body has \"kind\": \"alive\" only resets that clock, with no run and no share of the hourly rate. In the prompt {{due}} is the due time; {{event}}, {{repo}} and {{a.b}} (a path into the event's payload, like {{pull_request.title}}) fill from the event. An end time (until) is optional: at it the routine ends, with no run after it; to end it sooner call stop_routine with its id. A script routine (script, instead of a prompt) runs a shell command on the Mac on its trigger, with no agent and no tokens (30 s at most, its output cut at 4,000 characters), in one of two modes: post (its output is posted in this chat, and the owner gets a push when it exits non-zero) or wake (exit 0 stays silent; any other exit sends its output to you as the prompt); it is saved only once the owner approves its card. Each run that prompts you is guarded: a due time runs only when the owner wrote in this persona's chats since the last run, or on its first due time of a day, and every routine stops at the owner's runs-a-day cap; to watch something outside, use a wake script, a webhook or a GitHub event. A refusal starts with BAD_SCHEDULE, BAD_GITHUB, BAD_WEBHOOK, BAD_TRIGGER, BAD_NAME, BAD_PROMPT, BAD_SCRIPT, BAD_PLACE, BAD_UNTIL, TOO_MANY, NO_HOOKS or NOT_HERE.",
        "inputSchema": { "type": "object", "required": ["name"], "properties": {
            "name": { "type": "string", "description": "1 to 60 characters" },
            "when": { "type": "string", "description": "5-field cron in the Mac's local time (minute hour day-of-month month day-of-week; numbers, *, a-b, */n, lists), at least 15 minutes apart: 0 9 * * 1 is Mondays 09:00" },
            "github": { "type": "object", "required": ["repo", "events"], "properties": {
                "repo": { "type": "string", "description": "owner/name" },
                "events": { "type": "array", "items": { "type": "string" }, "description": "1 to 8, like pull_request (any action) or pull_request.opened" },
                "match": { "type": "object", "additionalProperties": { "type": "string" }, "description": "payload path to the text it must equal, like {\"workflow_run.conclusion\": \"failure\"}" }
            } },
            "webhook": { "type": "object", "required": ["secret"], "properties": {
                "secret": { "type": "string", "description": "the secret's name the sender signs with (A-Z, 0-9, _), like ALERTS_HOOK" },
                "silence": { "type": "integer", "minimum": 1, "maximum": 10080, "description": "minutes with no signed POST before one alert (a push and a line in this chat), and one line when they're back" }
            } },
            "mail": { "type": "boolean", "description": "run when mail arrives at your persona's catch-all inbox; its sender and subject follow the prompt" },
            "prompt": { "type": "string", "description": "what each run sends you; not for a script routine" },
            "script": { "type": "object", "required": ["run", "mode"], "properties": {
                "run": { "type": "string", "description": "the sh command line, 1 to 2,000 characters, run as is" },
                "mode": { "type": "string", "enum": ["post", "wake"], "description": "post: its output goes in the chat, a push on a non-zero exit; wake: a non-zero exit sends its output to you" }
            } },
            "place": { "type": "string", "enum": ["chat", "new_thread"] },
            "until": { "type": "string", "description": "optional end time, YYYY-MM-DD HH:MM in the Mac's local time: no run at or after it" }
        } }
    })
}

pub(super) const TOOL: Own = Own {
    name: "create_routine",
    shape,
    path: "/mcp/agent/routine",
    said: "",
};
