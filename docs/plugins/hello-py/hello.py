# A whole cooee plugin with a live screen: no frontend code, only JSON (screens-as-data, cooee#125). Python 3, no deps.
import json, os, threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

waves, changed = 0, threading.Condition()
def screen():
    return {"type": "screen", "id": "hello", "title": "Hello", "stream": "/screens/hello/stream", "blocks": [
        {"type": "text", "markdown": "Hi from **hello.py**. This screen is JSON; cooee draws it."},
        {"type": "stat", "id": "waves", "label": "waves today", "value": str(waves)},
        {"type": "button", "id": "wave", "label": "Wave", "style": "primary", "action": "wave"}]}
def ops(): return [{"op": "merge", "id": "waves", "fields": {"value": str(waves)}}]

class H(BaseHTTPRequestHandler):
    def head(self, kind):
        if self.headers.get("authorization") != "Bearer " + os.environ["TOKEN"]: return self.send_error(401)
        self.send_response(200); self.send_header("content-type", kind); self.end_headers(); return True
    def reply(self, body):
        if self.head("application/json"): self.wfile.write(json.dumps(body).encode())
    def do_GET(self):
        if not self.path.endswith("/stream"): return self.reply(screen())
        seen = -1 if self.head("text/event-stream") else None
        while seen is not None:
            with changed: changed.wait_for(lambda: waves != seen); seen = waves
            self.wfile.write(f"id: {seen}\ndata: {json.dumps({'seq': seen, 'ops': ops()})}\n\n".encode()); self.wfile.flush()
    def do_POST(self):
        global waves
        self.rfile.read(int(self.headers.get("content-length", 0)))
        if self.headers.get("authorization") == "Bearer " + os.environ["TOKEN"]:
            with changed: waves += 1; changed.notify_all()
        self.reply({"ops": ops()})

ThreadingHTTPServer(("127.0.0.1", int(os.environ.get("PORT", 4800))), H).serve_forever()
