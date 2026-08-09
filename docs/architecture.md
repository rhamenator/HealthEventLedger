# Architecture

HealthEventLedger is intentionally small and local.

```text
strict JSON -> normalization and validation -> SQLite event store
                                                |-> UTC daily summaries
                                                |-> clinician CSV
                                                `-> descriptive PDF
```

## Boundaries

- `domain.rs` owns the input contract, validation, normalization, digesting, and descriptive aggregation.
- `store.rs` owns the SQLite schema, transactions, duplicate suppression, and chronological reads.
- `report.rs` owns event-level CSV and the one-page descriptive PDF.
- `main.rs` provides explicit import, summary, and export commands.

The core library has no network client and no telemetry. SQLite is bundled for predictable local builds. Imports run in a transaction, and the unique source digest prevents the same normalized observation from being inserted twice.

The PDF is intentionally bounded to 24 UTC dates and directs readers to the companion CSV for additional dates. This keeps it a discussion summary instead of pretending to be a complete medical record.
