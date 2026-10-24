# TME Research & Optimization Log

## R001 — Retention event ID indexing

Status: Implemented, initial benchmark passed.

### Problem

Retention event insertion previously checked global ID
uniqueness by scanning all recorded retention events.

This made insertion increasingly expensive as history grew.

### Implemented approach

- Non-serialized BTreeSet index for retention event IDs.
- Lazy index reconstruction after deserialization.
- Retention history remains authoritative.
- Checked insertion and persistence validation.
- No change to the persistent JSON representation.

### Measurements

Release build, single-process measurements on the
development machine. Times in milliseconds.

| Events | Before | After |
| ---: | ---: | ---: |
| 1,000 | 0.831 | 0.388 |
| 3,000 | 3.208 | 1.036 |
| 6,000 | 15.832 | 1.467 |

At 6,000 events: approximately 10.8x faster insertion.

These are individual microbenchmark measurements,
not statistical performance guarantees.

### Research references

- Rust BTreeSet documentation:
  https://doc.rust-lang.org/std/collections/struct.BTreeSet.html
- Rust HashSet documentation:
  https://doc.rust-lang.org/std/collections/struct.HashSet.html
- Criterion.rs benchmarking:
  https://criterion-rs.github.io/book/

### Open questions

- Compare BTreeSet and HashSet on the same workloads.
- Measure peak RAM and indexing overhead.
- Benchmark larger histories and cold deserialization.
- Measure performance with multiple token histories.
- Add statistical benchmarking and regression detection.
- Verify compatibility for all supported loading paths.

### Decision

Retain BTreeSet provisionally until comparative benchmarks
justify an alternative. Do not infer long-term performance
from a single short benchmark.

## R003 — Persistence and index reconstruction

Status: Implemented; initial measurements completed.

### Problem

Memory::load validated retention history but discarded
the derived ID index. The first subsequent write then
repeated the same full-history traversal.

### Decision

Memory::load now prepares the ID index during mandatory
retention validation.

Direct Serde deserialization continues to initialize
without an index. Explicit preparation remains available.

Persistent JSON representation remains unchanged.

### Measurements

100,000 retention events across 100 tokens.
Individual release measurements in milliseconds.

| Operation | Before | After |
| --- | ---: | ---: |
| Load including validation | 41.754 | 41.511 |
| First write | 15.835 | 0.005 |
| Load + first write | 57.589 | 41.516 |

Observed end-to-end improvement: approximately 27.9%.

These are not statistically controlled measurements.

### Verification

- 104 Rust tests passed.
- File-loaded Memory contains a prepared ID index.
- Raw Serde deserialization retains lazy behavior.
- Persistent JSON remains equivalent.
- Existing evidence-graph validation remains enabled.

### Follow-up

- Repeat cold-load measurements statistically.
- Measure index overhead separately from event storage.
- Compare read-only startup against prepared startup.
- Benchmark larger historical datasets.
- Evaluate persistent storage alternatives only if justified.
