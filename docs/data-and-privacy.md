# Data and privacy

Health observations can be unusually sensitive. HealthEventLedger therefore keeps its working database and exports on the user's machine and does not include synchronization, analytics, accounts, advertising, or telemetry.

## Input contract

Each JSON object requires:

- `occurred_at`: an RFC 3339 timestamp, normalized to UTC
- `event_type`: `symptom`, `medication`, `trigger`, `sleep`, `activity`, or `note`
- `name`: a non-empty observation label

Optional fields are `severity` (0–10), `quantity` (finite number), `unit`, `duration_minutes`, `notes`, and `tags`. Tags are trimmed, lowercased, deduplicated, and sorted.

## Operational advice

- Treat database, CSV, and PDF files as private health material.
- Store exports only where you intend to share them.
- Review every export before giving it to another person.
- Delete local exports according to your own retention needs.
- Do not record an emergency here instead of seeking immediate help.

The included example is fictional. The repository ignores generated databases and exports so they are not accidentally committed.
