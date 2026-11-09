#!/bin/bash
# usage: api.sh METHOD PATH [JSON]
B="http://127.0.0.1:$P"
CK=$(python3 -I -c 'import json,sys; s=json.load(open(sys.argv[1])); print([c["value"] for c in s["cookies"] if c["name"]=="cooee"][0])' "$OUT/state.json")
echo "\$ curl -X $1 $2 ${3:+-d '$3'}"
if [ -n "$3" ]; then
  curl -s -w '\n[http %{http_code}]\n' -X "$1" -H "Cookie: cooee=$CK" -H 'content-type: application/json' -d "$3" "$B$2"
else
  curl -s -w '\n[http %{http_code}]\n' -X "$1" -H "Cookie: cooee=$CK" "$B$2"
fi
