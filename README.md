<p align="center">
  <img src=".github/assets/logo.png" alt="HealthEventLedger logo" width="220">
</p>

# HealthEventLedger

HealthEventLedger is a fast, local-first health event diary for recording structured observations and preparing concise material for a clinician conversation. It normalizes strict JSON into SQLite, builds chronological timelines and descriptive daily aggregates, and exports full-detail CSV plus a polished summary PDF.

It does **not** diagnose conditions, infer causes, or recommend treatment.

## What it records

- symptoms, medication, triggers, sleep, activity, and notes
- RFC 3339 timestamps normalized to UTC
- optional severity (0–10), quantity, unit, duration, notes, and tags
- a SHA-256 source digest that makes repeated imports idempotent

Only fictional data is included in this repository.

## Try it

```powershell
cargo run -- import health.db examples\synthetic-events.json
cargo run -- summary health.db
cargo run -- export-csv health.db clinician-timeline.csv
cargo run -- export-pdf health.db health-summary.pdf
```

Or run the in-memory demonstration:

```powershell
cargo run -- demo
```

The CSV preserves event-level detail for review. The PDF deliberately contains descriptive aggregates only. Dates in summaries are UTC calendar dates; the normalized event timestamp remains available in the CSV.

## Quality checks

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

See [Architecture](docs/architecture.md), [Data and privacy](docs/data-and-privacy.md), and [Clean-room notes](docs/clean-room.md).

## Status

This is an early, working implementation. It is suitable for experimenting with personal record keeping and export workflows, not for emergency care or clinical decision support. No license has been selected.
