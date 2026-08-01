# SPEC

## §G GOAL

Inference harness. Drive the local endpoint to test blackbox's own claims.

## §C CONSTRAINTS

- endpoint: `BBX_ENDPOINT`, default `http://192.168.0.181:11434` (24GB M5 Pro, measured).
- model: `gpt-oss:20b` MXFP4 — 24 layers, 8 KV heads, k/v len 64, sliding_window 128, ctx 131,072.
- MEASURED on that box: ctx 131,072 allocated, 11.98G of 24G resident, 100% GPU, ⊥ CPU spill.
- stdlib only. ⊥ pip install.

## §V INVARIANTS

V1: ∀ run records tokens SENT, latency, verdict. a trial w/o its cost is ⊥ evidence
V2: premise claim = model completes from a LENS pack a task it fails from the MONOLITH. unproven ∴ blackbox is an unmeasured optimization (`.:V60`)
V3: `num_ctx` passed PER REQUEST, ⊥ set globally. server default would allocate a full cache for every model
V4: N trials, hit rate reported. 1 sample ⊥ a result
V5: report what was EXAMINED — trials run, not only trials passed

## §T TASKS

id|status|task|cites
T1|x|`ask.sh` endpoint wrapper|V3
T2|x|`premise.py` monolith-vs-lens harness|V1,V2,V4
T3|.|run the premise gate on `itok` SPEC.md, record verdict in root §B|V2
T4|.|tool-schema entry cost measurement — 28,543 is inherited, ⊥ measured here|V1
