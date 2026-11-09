import sys, json, http.server
log = open(sys.argv[2], "a")
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get("content-length", 0)); body = self.rfile.read(n).decode()
        tok = self.path.rsplit("/", 1)[-1]
        log.write(json.dumps({"path": self.path, "headers": dict(self.headers), "body": body}) + "\n"); log.flush()
        code, msg = (410, '{"reason":"Unregistered"}') if tok.startswith("dead") or tok.startswith("dd") else (200, "")
        self.send_response(code); self.send_header("content-length", str(len(msg))); self.end_headers(); self.wfile.write(msg.encode())
http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
