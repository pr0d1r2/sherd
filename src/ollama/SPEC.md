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

## §T TASKS

id|status|task|cites
T1|x|`generate` + `Reply` w/ server-counted tokens|V1,V2,V3
T2|x|`rust_block` fence extraction + unterminated-fence guard|V4
T3|.|retry w/ backoff on transport failure, bounded|V3
