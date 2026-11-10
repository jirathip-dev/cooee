# A whole cooee plugin with a screen: no frontend code, only JSON (screens-as-data, cooee#125). Python 3, no deps.
import json, os
from http.server import BaseHTTPRequestHandler, HTTPServer

waves = 0
def screen():
    return {"type": "screen", "id": "hello", "title": "Hello", "blocks": [
        {"type": "text", "markdown": "Hi from **hello.py**. This screen is JSON; cooee draws it."},
        {"type": "stat", "id": "waves", "label": "waves today", "value": str(waves)},
        {"type": "button", "id": "wave", "label": "Wave", "style": "primary", "action": "wave"}]}

class H(BaseHTTPRequestHandler):
    def reply(self, body):
        if self.headers.get("authorization") != "Bearer " + os.environ["TOKEN"]: return self.send_error(401)
        b = json.dumps(body).encode(); self.send_response(200)
        self.send_header("content-type", "application/json"); self.end_headers(); self.wfile.write(b)
    def do_GET(self): self.reply(screen())
    def do_POST(self):
        global waves; waves += self.headers.get("authorization") == "Bearer " + os.environ["TOKEN"]
        self.reply({"ops": [{"op": "merge", "id": "waves", "fields": {"value": str(waves)}}]})

HTTPServer(("127.0.0.1", int(os.environ.get("PORT", 4800))), H).serve_forever()
