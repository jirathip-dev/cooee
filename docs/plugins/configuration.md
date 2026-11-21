# Configuration

One `[[plugins]]` entry per plugin in `config.toml`:

```toml
[[plugins]]
name = "hello"                    # its URL: /p/hello (lowercase letters, digits and -)
label = "Hello"                   # the tab's label
url = "http://127.0.0.1:4800"     # its backend: plain http, on this machine or the tailnet
module = "/tab.js"                # its UI module, a path on the backend
token = "a-long-random-secret"    # shared with the backend; at least 16 characters
```

`cooee serve` refuses to start when an entry breaks a rule (no token, two plugins of one name, …).
