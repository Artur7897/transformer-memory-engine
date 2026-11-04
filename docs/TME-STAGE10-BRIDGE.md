# TME – Integration Stage 10: Abstraction / Reflection Bridge

This revision starts from the **complete Stage 9 source archive**.
It adds four commands to `text-ingestion-bridge`, with no new state format.

## Commands

Run: `cargo run --bin text-ingestion-bridge -- <command> <state-dir> <uuid> <serial> <payload.json>`

- `abstraction-preview` (read-only): payload `{"source_token_ids":["s1","s2"], "proposed_text":"...", "proposed_source":"..."}`. Validates that source tokens are eligible; **does not verify meaning**.
- `abstraction-commit` (atomic): payload `{"request":{... same as preview ...}, "commit":{"anchor_id":"new-anchor","token_id":"new-abstract","anchor_vector":[...],"text_vector":[...],"review_reference":"review:..."}}`. Only in ACTIVE, preserves all source segments and links. `review_reference` is audit metadata, **not proof of review**. Caller/Soul is responsible for an authorized semantic review.
- `maintenance-assess` (read-only): payload `{"mode":"reflection","signal":"routine","budget":20}` in ACTIVE, or `{"mode":"sleep","signal":"routine","budget":20}` in SLEEP. Shared core, different lifecycle profile.
- `reflection-pass` (atomic): payload `{"budget":20}`; ACTIVE only; acknowledges previously recorded operation counters under existing rules, does **not** delete or semantically merge tokens.

**Boundaries:** no automatic Qwen invocation; no ToolGateway or network access; SLEEP lifecycle is not exposed through the bridge by this stage. This is not end-to-end feature completion.

## Safety / validation

- Use a fresh throwaway `.tme` file with `create` for initial testing; never point this experimental bridge at Lucy or Academy production state.
- First perform `cargo check --bin text-ingestion-bridge` and `cargo test --lib` on the Workspace Mini.
- No Cargo toolchain was available in the artifact creation environment, so compilation and test results are **not claimed**.
