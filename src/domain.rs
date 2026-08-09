use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Symptom,
    Medication,
    Trigger,
    Sleep,
    Activity,
    Note,
}

impl EventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Symptom => "symptom",
            Self::Medication => "medication",
            Self::Trigger => "trigger",
            Self::Sleep => "sleep",
            Self::Activity => "activity",
            Self::Note => "note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEvent {
    pub occurred_at: String,
    pub event_type: EventType,
    pub name: String,
    #[serde(default)]
    pub severity: Option<u8>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<u32>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthEvent {
    pub occurred_at_ms: i64,
    pub occurred_at: String,
    pub event_type: EventType,
    pub name: String,
    pub severity: Option<u8>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub duration_minutes: Option<u32>,
    pub notes: String,
    pub tags: Vec<String>,
    pub source_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailySummary {
    pub date_utc: String,
    pub event_count: usize,
    pub symptom_count: usize,
    pub maximum_severity: Option<u8>,
    pub average_severity: Option<f64>,
    pub medication_events: usize,
    pub trigger_events: usize,
    pub recorded_sleep_minutes: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NormalizeError {
    #[error("occurred_at must be an RFC 3339 timestamp")]
    InvalidTimestamp,
    #[error("event name cannot be empty")]
    EmptyName,
    #[error("severity must be between 0 and 10")]
    InvalidSeverity,
    #[error("quantity must be finite")]
    InvalidQuantity,
}

impl TryFrom<ImportEvent> for HealthEvent {
    type Error = NormalizeError;

    fn try_from(mut raw: ImportEvent) -> Result<Self, Self::Error> {
        let timestamp = OffsetDateTime::parse(raw.occurred_at.trim(), &Rfc3339)
            .map_err(|_| NormalizeError::InvalidTimestamp)?;
        let name = raw.name.trim().to_owned();
        if name.is_empty() {
            return Err(NormalizeError::EmptyName);
        }
        if raw.severity.is_some_and(|severity| severity > 10) {
            return Err(NormalizeError::InvalidSeverity);
        }
        if raw.quantity.is_some_and(|quantity| !quantity.is_finite()) {
            return Err(NormalizeError::InvalidQuantity);
        }
        raw.tags = raw
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect();
        raw.tags.sort();
        raw.tags.dedup();
        let occurred_at = timestamp
            .format(&Rfc3339)
            .expect("RFC 3339 formatting is infallible for valid timestamps");
        let occurred_at_ms = (timestamp.unix_timestamp_nanos() / 1_000_000) as i64;
        let canonical = serde_json::to_vec(&(
            occurred_at_ms,
            raw.event_type,
            &name,
            raw.severity,
            raw.quantity.map(f64::to_bits),
            &raw.unit,
            raw.duration_minutes,
            raw.notes.trim(),
            &raw.tags,
        ))
        .expect("normalized fields serialize");
        let source_digest = format!("{:x}", Sha256::digest(canonical));
        Ok(Self {
            occurred_at_ms,
            occurred_at,
            event_type: raw.event_type,
            name,
            severity: raw.severity,
            quantity: raw.quantity,
            unit: raw
                .unit
                .map(|unit| unit.trim().to_owned())
                .filter(|unit| !unit.is_empty()),
            duration_minutes: raw.duration_minutes,
            notes: raw.notes.trim().to_owned(),
            tags: raw.tags,
            source_digest,
        })
    }
}

pub fn normalize_json(json: &str) -> Result<Vec<HealthEvent>, ImportError> {
    let raw: Vec<ImportEvent> = serde_json::from_str(json)?;
    raw.into_iter()
        .enumerate()
        .map(|(index, event)| {
            HealthEvent::try_from(event).map_err(|source| ImportError::Event { index, source })
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("input is not a valid JSON event array: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event {index} is invalid: {source}")]
    Event {
        index: usize,
        source: NormalizeError,
    },
}

pub fn daily_summaries(events: &[HealthEvent]) -> Vec<DailySummary> {
    let mut groups: BTreeMap<String, Vec<&HealthEvent>> = BTreeMap::new();
    for event in events {
        let date = event.occurred_at.get(..10).unwrap_or(&event.occurred_at);
        groups.entry(date.to_owned()).or_default().push(event);
    }
    groups
        .into_iter()
        .map(|(date_utc, events)| {
            let severities: Vec<u8> = events.iter().filter_map(|event| event.severity).collect();
            let average_severity = (!severities.is_empty()).then(|| {
                severities.iter().map(|value| *value as f64).sum::<f64>() / severities.len() as f64
            });
            DailySummary {
                date_utc,
                event_count: events.len(),
                symptom_count: events
                    .iter()
                    .filter(|event| event.event_type == EventType::Symptom)
                    .count(),
                maximum_severity: severities.iter().copied().max(),
                average_severity,
                medication_events: events
                    .iter()
                    .filter(|event| event.event_type == EventType::Medication)
                    .count(),
                trigger_events: events
                    .iter()
                    .filter(|event| event.event_type == EventType::Trigger)
                    .count(),
                recorded_sleep_minutes: events
                    .iter()
                    .filter(|event| event.event_type == EventType::Sleep)
                    .filter_map(|event| event.duration_minutes)
                    .sum(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(severity: Option<u8>) -> ImportEvent {
        ImportEvent {
            occurred_at: "2026-01-02T10:30:00-05:00".into(),
            event_type: EventType::Symptom,
            name: " Head discomfort ".into(),
            severity,
            quantity: None,
            unit: None,
            duration_minutes: Some(45),
            notes: " synthetic note ".into(),
            tags: vec!["Work".into(), " work ".into()],
        }
    }

    #[test]
    fn normalizes_timestamp_text_and_tags() {
        let event = HealthEvent::try_from(raw(Some(4))).unwrap();
        assert_eq!(event.occurred_at, "2026-01-02T10:30:00-05:00");
        assert_eq!(event.name, "Head discomfort");
        assert_eq!(event.tags, ["work"]);
        assert_eq!(event.source_digest.len(), 64);
    }

    #[test]
    fn rejects_out_of_range_severity() {
        assert_eq!(
            HealthEvent::try_from(raw(Some(11))).unwrap_err(),
            NormalizeError::InvalidSeverity
        );
    }

    #[test]
    fn summaries_are_descriptive_only() {
        let first = HealthEvent::try_from(raw(Some(4))).unwrap();
        let mut second_raw = raw(Some(8));
        second_raw.occurred_at = "2026-01-02T18:00:00Z".into();
        let second = HealthEvent::try_from(second_raw).unwrap();
        let summaries = daily_summaries(&[first, second]);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].maximum_severity, Some(8));
        assert_eq!(summaries[0].average_severity, Some(6.0));
    }
}
