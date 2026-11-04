# Stage 12 — Bind Abstraction commit to the reviewed proposal

## Problem
Before Stage 12, `abstraction-commit` accepted an arbitrary nonempty `review_reference` but did not prove that the proposed text and source texts/identifiers were identical to the material shown during `abstraction-preview`. A changed proposal could be committed under an old review reference.

## Contract
- `abstraction-preview` now returns `review_digest`, a SHA-256 digest of a versioned JSON tuple of the ordered `(token_id, text, source)` records plus proposed (trimmed) text and proposed source.
- The external reviewer must review the preview and pass **that exact** `review_digest` back as `commit.reviewed_digest`.
- On commit, TME recomputes the digest from its current source content. If source order/content/provenance or proposed content changed, the transaction fails with no state mutation.
- New abstraction records persist the digest. Legacy records without this field can still deserialize (`serde(default)`).
- `review_reference` and the digest are *audit/integrity* metadata. They do not prove the reviewer was authorized or the text is true; authorization and semantic review belong to the Soul/LLM workflow.
- Existing original tokens remain unmodified and accessible.

## Bridge payload example
The `abstraction-commit` JSON `commit` object must now include:

```json
{
  "anchor_id": "abstract-doc",
  "token_id": "abstract-one",
  "anchor_vector": [1.0, 0.0],
  "text_vector": [1.0, 1.0],
  "review_reference": "review:academy:001",
  "reviewed_digest": "<the exact 64-character review_digest returned by abstraction-preview>"
}
```

This is a Stage-12 interface change. Do not send a placeholder digest.

## Checks
The new Rust unit test exercises altered text, changed source ordering, rejection without mutation, and successful commit. Actual `cargo test` must still run on the Workspace Mini because this preparation environment has no Cargo/Rust compiler.
