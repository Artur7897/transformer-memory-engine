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
