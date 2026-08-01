#!/usr/bin/env bash
# Thin wrapper over the local endpoint. Reads the context from stdin so a
# lens pack pipes straight in:
#
#   bbx lens src/fed | scripts/ask.sh "which rule forbids skipping levels?"
#
# num_ctx is passed PER REQUEST (scripts §V.3) -- setting it globally makes
# every model allocate a full cache whether it needs one or not.
set -euo pipefail

ENDPOINT="${BBX_ENDPOINT:-http://192.168.0.181:11434}"
MODEL="${BBX_MODEL:-gpt-oss:20b}"
NUM_CTX="${BBX_NUM_CTX:-131072}"

question="${1:-summarise this}"

ctx="$(mktemp)"; payload="$(mktemp)"
trap 'rm -f "$ctx" "$payload"' EXIT
cat >"$ctx"

# Build the JSON in python (the context is arbitrary text and must be
# escaped properly); read it from a FILE, since stdin is not available --
# that is what the first version of this script got wrong.
python3 -c '
import json, sys
ctx_path, question, num_ctx, model = sys.argv[1:5]
with open(ctx_path, encoding="utf-8") as fh:
    context = fh.read()
json.dump({
    "model": model,
    "prompt": f"{context}\n\n---\n{question}\n",
    "stream": False,
    "options": {"num_ctx": int(num_ctx), "temperature": 0},
}, sys.stdout)
' "$ctx" "$question" "$NUM_CTX" "$MODEL" >"$payload"

curl -s --max-time 900 "$ENDPOINT/api/generate" \
     -H 'Content-Type: application/json' --data-binary "@$payload" \
  | python3 -c '
import json, sys
d = json.load(sys.stdin)
print(d.get("response", ""))
pe = d.get("prompt_eval_count")
ev = d.get("eval_count")
ms = (d.get("total_duration") or 0) // 1_000_000
sys.stderr.write("\n[prompt_eval={} eval={} total_ms={}]\n".format(pe, ev, ms))
'
