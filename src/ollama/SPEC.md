# SPEC

## §G GOAL

Local inference endpoint. Sole call site for `ureq` + `serde_json`.

## §C CONSTRAINTS

- plain HTTP to LAN. ⊥ TLS, ⊥ cloud, ⊥ key. `ureq` default-features off = no cert chain.
- feature-gated `ollama`. `--no-default-features` leaves the deterministic core (`.:V18`).

## §V INVARIANTS

V1: `num_ctx` PER REQUEST, ⊥ global. global allocates a full cache for every model
V2: reply carries prompt tokens the SERVER counted — the real number, ⊥ our estimate
V3: transport failure → `Err`, ⊥ empty success (`.:V20`)
V4: `rust_block` takes LONGEST fenced block. unterminated fence → return input whole, ⊥ hang, ⊥ panic
V5: ∀ call reports EXPECTED cost before sending & ACTUAL after. a prediction nobody checks is decoration
V6: eta reports BOTH bounds — cold & cached. prefill is ~100x faster on a hot prefix (`.:R14`) & in practice the prefix usually IS hot ∴ a cold-only eta reads ~90% high
V7: escalation vs the COLD estimate — notice @ 1x, again @ 2x, WARN @ 3x, ABORT @ 4x. over-estimating is the direction that ⊥ false-alarm
V8: rates LEARNED from telemetry, EWMA 1/4, persisted to `.bbx-pace` ∴ a single-call run benefits from what earlier runs measured
V9: a CACHE HIT ⊥ evidence about cold prefill speed. folding it into the learned rate collapses it & every later eta w/ it
V10: cache detected by an ABSOLUTE bound (>3,000 tok/s). cold spans 284-1,519 measured ∴ a threshold RELATIVE to the learned rate rises w/ it & stops firing (B4)
V11: ladder = notice 1x · notice 2x · WARN 5x · hard stop 10x. a ladder tight enough to abort a WORKING call also prevents the measurement that would fix the estimate (B5)
V12: an ABORT still teaches — record what it managed as a FLOOR for that step kind, ⊥ averaged w/ the estimate that was already wrong
V13: telemetry RETAINED raw, ⊥ folded away. an average cannot be re-derived into a median, a percentile or a per-size fit; samples can become all three
V14: prefill rate is a fn of SIZE — 1,519 @ 7k · 1,233 @ 15k · 941 @ 28k (`.:R17`) ∴ bucketed, ⊥ one scalar wrong at both ends
V15: `load_duration` > 500ms = COLD endpoint. disk time, ⊥ prefill ∴ subtracted before learning & reported
V17: sampling is EXPLICIT (`Sampling`), ⊥ hardcoded — it changes what a call MEANS. @ temp 0 model deterministic (`.:V17`) ∴ 2 calls on 1 prompt = 1 call run twice. any temp > 0 ! carry a SEED — diversity w/o reproducibility makes a failure impossible to re-examine & "it was different that time" is the explanation `.:V17` refuses
V16: IO goes through `Transport` ∴ a caller can substitute one that fails on demand. w/o a seam there is nothing to wrap & the model faked the transport instead (B7). `Http` is the real one; the core stays networkless w/o the feature

## §T TASKS

id|status|task|cites
T1|x|`generate` + `Reply` w/ server-counted tokens|V1,V2,V3
T2|x|`rust_block` fence extraction + unterminated-fence guard|V4
T3|.|retry w/ bounded backoff around `Transport::post`|V16
T4|x|streaming + pace model + escalation guards|V5,V7
T5|x|learned rates persisted, cache-hit detection|V8,V9,V10
T6|x|raw telemetry retained, deduped, bounded; per-size derived rates|V13,V14
T7|x|cold-endpoint detection, excluded from rate learning|V15
T8|.|probe the endpoint's tier and warn when falling back|V3
T9|x|`Transport` seam + `generate_via`|V16
T10|x|`Sampling` threaded to the request body, verified via a spy transport|V17,V16

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`bbx tdd` silent 40-90s per step — `eprintln!` fires only AFTER the reply|stream (`"stream": true`) + `->` line before the call
B2|2026-08-01|`ask` bypassed the pace path ∴ two report formats for one operation|`ask` routes through `generate_with` like every other call
B3|2026-08-01|learned rates lived in process statics ∴ every single-call run started from the seed & learned nothing lasting|persist to `.bbx-pace`
B4|2026-08-01|cache detection compared observed prefill to 8x the LEARNED rate ∴ as the rate climbed the bar climbed w/ it & the flag never fired on obvious hits|absolute bound, 3,000 tok/s, derived from the measured cold span
B5|2026-08-01|4x abort killed a HEALTHY run @ 42s. eta 11s because `gen_est` knew nothing of the 2,614 REASONING tokens `gpt-oss` emits before its first output token. worse: `eval_count` arrives only on the `done` frame ∴ every abort taught NOTHING & the next run predicted just as badly|ladder → 5x warn / 10x stop; abort records a FLOOR for that step kind. a guard that prevents its own correction is a trap
B6|2026-08-01|`trim_kind` shipped w/ a stub helper returning 0 ∴ it never trimmed & state would grow unbounded. build was green — a no-op guard compiles fine|implemented + 2 tests: bounds to n, keeps newest, leaves other kinds alone, no-op under the limit
B7|2026-08-01|`apply` wrote `_generate_stub` — a fake transport returning empty on an unreachable host, named "stub", documented "for the purposes of the test suite", `_`-prefixed like B3. gates red, discarded. asked to add retry AROUND real IO, it replaced the IO|`generate` needs a seam — a transport param or trait — before a retry wrapper is testable. 3rd self-documented stub of the run, 2nd `_` evasion
