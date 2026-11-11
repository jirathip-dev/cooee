# #140: a fixture `drover` for the app's end-to-end run (serve.sh): #125's Queue and Events as screens, the queue's
# stream playing queue-stream.json's ops a few seconds in, and `pause` taking it down for 20 s (the offline case).
import json, os, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

down_until = 0.0
def fixture(name): return json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), name)))

class H(BaseHTTPRequestHandler):
    def start(self, status, kind):
        self.send_response(status); self.send_header("content-type", kind); self.end_headers()
    def do_GET(self):
        if self.headers.get("authorization") != "Bearer " + os.environ["TOKEN"]: return self.send_error(401)
        if time.time() < down_until: return self.send_error(503)
        name = self.path.split("?")[0].removeprefix("/screens/")
        if not name.endswith("/stream"):
            self.start(200, "application/json"); return self.wfile.write(json.dumps(fixture(name + ".json")).encode())
        self.start(200, "text/event-stream")
        events = fixture("queue-stream.json") if name == "queue/stream" else []
        for e in events:
            time.sleep(4)
            self.wfile.write(f"id: {e['seq']}\ndata: {json.dumps(e)}\n\n".encode()); self.wfile.flush()
        while time.time() >= down_until: time.sleep(1)
    def do_POST(self):
        global down_until
        action = json.loads(self.rfile.read(int(self.headers.get("content-length", 0))) or b"{}").get("action")
        if action == "pause": down_until = time.time() + 20
        self.start(200, "application/json"); self.wfile.write(json.dumps({"ops": [], "toast": f"{action}: done"}).encode())

ThreadingHTTPServer(("127.0.0.1", int(os.environ["PORT"])), H).serve_forever()
