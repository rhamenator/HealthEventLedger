use std::path::Path;

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::{EventType, HealthEvent};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored event type is invalid: {0}")]
    InvalidEventType(String),
}

pub struct HealthStore {
    connection: Connection,
}

impl HealthStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS health_events (
                id INTEGER PRIMARY KEY,
                occurred_at_ms INTEGER NOT NULL,
                occurred_at TEXT NOT NULL,
                event_type TEXT NOT NULL,
                name TEXT NOT NULL,
                severity INTEGER,
                quantity REAL,
                unit TEXT,
                duration_minutes INTEGER,
                notes TEXT NOT NULL,
                tags_json TEXT NOT NULL,
                source_digest TEXT NOT NULL UNIQUE
            );
            CREATE INDEX IF NOT EXISTS health_events_timeline
                ON health_events (occurred_at_ms, id);",
        )?;
        Ok(Self { connection })
    }

    pub fn import(&mut self, events: &[HealthEvent]) -> Result<usize, StoreError> {
        let transaction = self.connection.transaction()?;
        let mut inserted = 0;
        for event in events {
            inserted += transaction.execute(
                "INSERT OR IGNORE INTO health_events
                 (occurred_at_ms, occurred_at, event_type, name, severity, quantity,
                  unit, duration_minutes, notes, tags_json, source_digest)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    event.occurred_at_ms,
                    event.occurred_at,
                    event.event_type.as_str(),
                    event.name,
                    event.severity,
                    event.quantity,
                    event.unit,
                    event.duration_minutes,
                    event.notes,
                    serde_json::to_string(&event.tags)?,
                    event.source_digest,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn timeline(&self) -> Result<Vec<HealthEvent>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT occurred_at_ms, occurred_at, event_type, name, severity, quantity,
                    unit, duration_minutes, notes, tags_json, source_digest
             FROM health_events ORDER BY occurred_at_ms, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<u8>>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<u32>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
            ))
        })?;
        rows.map(|row| {
            let (
                occurred_at_ms,
                occurred_at,
                event_type,
                name,
                severity,
                quantity,
                unit,
                duration_minutes,
                notes,
                tags_json,
                source_digest,
            ) = row?;
            Ok(HealthEvent {
                occurred_at_ms,
                occurred_at,
                event_type: parse_event_type(&event_type)?,
                name,
                severity,
                quantity,
                unit,
                duration_minutes,
                notes,
                tags: serde_json::from_str(&tags_json)?,
                source_digest,
            })
        })
        .collect()
    }
}

fn parse_event_type(value: &str) -> Result<EventType, StoreError> {
    match value {
        "symptom" => Ok(EventType::Symptom),
        "medication" => Ok(EventType::Medication),
        "trigger" => Ok(EventType::Trigger),
        "sleep" => Ok(EventType::Sleep),
        "activity" => Ok(EventType::Activity),
        "note" => Ok(EventType::Note),
        _ => Err(StoreError::InvalidEventType(value.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize_json;

    #[test]
    fn sqlite_import_is_idempotent_and_sorted() {
        let input = r#"[{"occurred_at":"2026-01-02T12:00:00Z","event_type":"note","name":"Second"},{"occurred_at":"2026-01-01T12:00:00Z","event_type":"note","name":"First"}]"#;
        let events = normalize_json(input).unwrap();
        let mut store = HealthStore::in_memory().unwrap();
        assert_eq!(store.import(&events).unwrap(), 2);
        assert_eq!(store.import(&events).unwrap(), 0);
        let timeline = store.timeline().unwrap();
        assert_eq!(timeline[0].name, "First");
        assert_eq!(timeline[1].name, "Second");
    }
}
