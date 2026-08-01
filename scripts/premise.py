#!/usr/bin/env python3
"""The premise gate (root V60, scripts V2).

blackbox's whole claim is that a small local model completes from a LENS
pack a task it fails from the MONOLITH. Until that is measured, blackbox is
an unmeasured optimization -- itok's own B8, at project scale.

This runs the same question under both conditions, N trials each, and
reports cost and hit rate for both. It does not decide whether the claim
holds; it produces the number that decides it.

    scripts/premise.py \\
        --monolith ../itok/SPEC.md \\
        --lens <(cd .. && blackbox/target/debug/bbx lens src/fed) \\
        --question "which invariant forbids skipping federation levels?" \\
        --expect "depth" --expect "+1" \\
        --trials 3

stdlib only -- no pip install.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field

ENDPOINT = os.environ.get("BBX_ENDPOINT", "http://192.168.0.181:11434")
MODEL = os.environ.get("BBX_MODEL", "gpt-oss:20b")
NUM_CTX = int(os.environ.get("BBX_NUM_CTX", "131072"))

# Measured harness overhead: system prompt + tool schemas, resident before
# any file loads and re-billed every turn.
ENTRY_COST = 28_543

ITOK = os.environ.get(
    "BBX_ITOK", os.path.expanduser("~/projects/itok/target/release/itok")
)


def count_tokens(text: str) -> tuple[int, str]:
    """Real tokenizer count when itok is available; labelled either way.

    Never returns a bare number -- the method travels with it (tokens V1),
    because bytes/4 measured 48% low on caveman-encoded text.
    """
    if shutil.which(ITOK) or os.path.isfile(ITOK):
        try:
            proc = subprocess.run(
                [ITOK, "estimate", "--bpe", "--format", "json", "-"],
                input=text, capture_output=True, text=True, timeout=120, check=False,
            )
            for line in proc.stdout.splitlines():
                rec = json.loads(line)
                return int(rec["tokens"]), rec.get("method", "o200k")
        except (OSError, ValueError, KeyError, subprocess.SubprocessError):
            pass
    return len(text) // 4, "bytes/4 (ESTIMATE -- not a gate)"


@dataclass
class Trial:
    ok: bool
    prompt_tokens: int
    eval_tokens: int
    ms: int
    answer: str


@dataclass
class Condition:
    name: str
    context: str
    trials: list[Trial] = field(default_factory=list)

    @property
    def hits(self) -> int:
        return sum(1 for t in self.trials if t.ok)


def ask(context: str, question: str) -> tuple[str, int, int, int]:
    payload = json.dumps({
        "model": MODEL,
        "prompt": f"{context}\n\n---\n{question}\n",
        "stream": False,
        # Per request, never global (scripts V3).
        "options": {"num_ctx": NUM_CTX, "temperature": 0},
    }).encode()
    req = urllib.request.Request(
        f"{ENDPOINT}/api/generate", data=payload,
        headers={"Content-Type": "application/json"},
    )
    start = time.time()
    with urllib.request.urlopen(req, timeout=900) as resp:
        d = json.load(resp)
    ms = int((time.time() - start) * 1000)
    return (
        d.get("response", ""),
        int(d.get("prompt_eval_count") or 0),
        int(d.get("eval_count") or 0),
        ms,
    )


def run(cond: Condition, question: str, expect: list[str], trials: int) -> None:
    for i in range(trials):
        try:
            answer, ptok, etok, ms = ask(cond.context, question)
        except (urllib.error.URLError, TimeoutError) as e:
            # V5: a failed trial is reported, never silently dropped.
            print(f"  {cond.name} trial {i+1}: TRANSPORT FAILURE {e}", file=sys.stderr)
            cond.trials.append(Trial(False, 0, 0, 0, f"<failed: {e}>"))
            continue
        low = answer.lower()
        ok = all(e.lower() in low for e in expect)
        cond.trials.append(Trial(ok, ptok, etok, ms, answer))
        print(f"  {cond.name} trial {i+1}/{trials}: "
              f"{'HIT ' if ok else 'MISS'} prompt={ptok} eval={etok} {ms}ms")


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--monolith", required=True, help="file: the whole spec")
    p.add_argument("--lens", required=True, help="file: just the node's pack")
    p.add_argument("--question", required=True)
    p.add_argument("--expect", action="append", default=[],
                   help="substring the answer must contain; repeatable")
    p.add_argument("--trials", type=int, default=3)
    a = p.parse_args()

    if not a.expect:
        print("premise: --expect is required -- a trial with no pass "
              "condition measures nothing", file=sys.stderr)
        return 2

    conds = [
        Condition("monolith", open(a.monolith, encoding="utf-8").read()),
        Condition("lens    ", open(a.lens, encoding="utf-8").read()),
    ]

    print(f"endpoint {ENDPOINT} · model {MODEL} · num_ctx {NUM_CTX:,}")
    print(f"question: {a.question}")
    print(f"expect:   {a.expect}\n")

    for c in conds:
        tok, method = count_tokens(c.context)
        work = max(0, NUM_CTX - ENTRY_COST)
        print(f"{c.name}: {tok:,} tok ({method}) · "
              f"{100*tok/work:.1f}% of {work:,} working budget")
    print()

    for c in conds:
        run(c, a.question, a.expect, a.trials)

    print(f"\n{'condition':10} {'hits':>7} {'ctx tok':>9} {'mean ms':>9}")
    for c in conds:
        tok, _ = count_tokens(c.context)
        done = [t for t in c.trials if t.ms]
        mean = sum(t.ms for t in done) // len(done) if done else 0
        print(f"{c.name:10} {c.hits:>3}/{len(c.trials):<3} {tok:>9,} {mean:>9,}")

    # V5: say what was examined, not only what passed.
    print(f"\n  {sum(len(c.trials) for c in conds)} trials run across "
          f"{len(conds)} conditions")
    mono, lens = conds[0], conds[1]
    if lens.hits > mono.hits:
        print("  VERDICT: lens beats monolith -- premise supported on this task")
    elif lens.hits == mono.hits:
        print("  VERDICT: tie -- this task does not discriminate. "
              "A task the monolith already passes cannot test the claim.")
    else:
        print("  VERDICT: monolith beats lens -- premise NOT supported here")
    return 0


if __name__ == "__main__":
    sys.exit(main())
