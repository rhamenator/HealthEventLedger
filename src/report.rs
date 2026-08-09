use std::fmt::Write as _;

use serde::Serialize;

use crate::{DailySummary, HealthEvent, daily_summaries};

pub fn clinician_csv(events: &[HealthEvent]) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    for event in events {
        writer.serialize(CsvRow::from(event))?;
    }
    let bytes = writer.into_inner().map_err(|error| error.into_error())?;
    Ok(String::from_utf8(bytes).expect("CSV fields are UTF-8"))
}

#[derive(Serialize)]
struct CsvRow<'a> {
    occurred_at: &'a str,
    event_type: &'a str,
    name: &'a str,
    severity_0_to_10: Option<u8>,
    quantity: Option<f64>,
    unit: Option<&'a str>,
    duration_minutes: Option<u32>,
    notes: &'a str,
    tags: String,
}

impl<'a> From<&'a HealthEvent> for CsvRow<'a> {
    fn from(event: &'a HealthEvent) -> Self {
        Self {
            occurred_at: &event.occurred_at,
            event_type: event.event_type.as_str(),
            name: &event.name,
            severity_0_to_10: event.severity,
            quantity: event.quantity,
            unit: event.unit.as_deref(),
            duration_minutes: event.duration_minutes,
            notes: &event.notes,
            tags: event.tags.join("; "),
        }
    }
}

pub fn clinician_pdf(events: &[HealthEvent]) -> Vec<u8> {
    let summaries = daily_summaries(events);
    designed_pdf(events.len(), &summaries)
}

fn summary_line(summary: &DailySummary) -> String {
    format!(
        "{} | events {} | symptoms {} | max severity {} | meds {} | sleep {} min",
        summary.date_utc,
        summary.event_count,
        summary.symptom_count,
        summary
            .maximum_severity
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".into()),
        summary.medication_events,
        summary.recorded_sleep_minutes,
    )
}

fn designed_pdf(event_count: usize, summaries: &[DailySummary]) -> Vec<u8> {
    let mut content = String::from("0.07 0.31 0.25 rg 0 700 612 92 re f ");
    text(
        &mut content,
        "F2",
        20,
        48,
        752,
        "Health event summary",
        "1 1 1",
    );
    text(
        &mut content,
        "F1",
        10,
        48,
        731,
        "A descriptive timeline for discussion - not a diagnosis",
        "0.9 0.96 0.93",
    );
    content.push_str("0.94 0.97 0.95 rg 48 622 245 58 re f 0.94 0.97 0.95 rg 319 622 245 58 re f ");
    text(
        &mut content,
        "F2",
        20,
        64,
        648,
        &event_count.to_string(),
        "0.07 0.31 0.25",
    );
    text(
        &mut content,
        "F1",
        9,
        64,
        634,
        "recorded events",
        "0.3 0.38 0.35",
    );
    text(
        &mut content,
        "F2",
        20,
        335,
        648,
        &summaries.len().to_string(),
        "0.07 0.31 0.25",
    );
    text(
        &mut content,
        "F1",
        9,
        335,
        634,
        "UTC dates represented",
        "0.3 0.38 0.35",
    );
    text(
        &mut content,
        "F2",
        13,
        48,
        584,
        "Daily aggregates",
        "0.12 0.18 0.16",
    );
    content.push_str("0.76 0.83 0.79 RG 0.6 w 48 574 m 564 574 l S ");
    let mut y = 552;
    for summary in summaries.iter().take(24) {
        text(
            &mut content,
            "F1",
            9,
            48,
            y,
            &summary_line(summary),
            "0.12 0.18 0.16",
        );
        content.push_str(&format!(
            "0.9 0.93 0.91 RG 0.4 w 48 {} m 564 {} l S ",
            y - 7,
            y - 7
        ));
        y -= 18;
    }
    if summaries.len() > 24 {
        text(
            &mut content,
            "F1",
            9,
            48,
            y,
            "Additional dates are available in the companion CSV export.",
            "0.3 0.38 0.35",
        );
    }
    content.push_str("0.99 0.96 0.88 rg 48 82 516 48 re f ");
    text(
        &mut content,
        "F2",
        9,
        62,
        108,
        "Clinical boundary",
        "0.38 0.25 0.08",
    );
    text(
        &mut content,
        "F1",
        9,
        62,
        92,
        "No diagnosis, causal inference, or treatment recommendation is made.",
        "0.38 0.25 0.08",
    );
    content.push_str("0.78 0.82 0.8 RG 0.5 w 48 54 m 564 54 l S ");
    text(
        &mut content,
        "F1",
        8,
        48,
        38,
        "Generated from user-recorded events",
        "0.38 0.45 0.42",
    );
    text(&mut content, "F1", 8, 526, 38, "Page 1", "0.38 0.45 0.42");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_owned(),
    ];
    let mut output = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = output.len();
    output.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    output
}

fn text(content: &mut String, font: &str, size: u8, x: i32, y: i32, value: &str, color: &str) {
    let _ = write!(
        content,
        "BT {color} rg /{font} {size} Tf 1 0 0 1 {x} {y} Tm ({}) Tj ET ",
        escape_pdf(value)
    );
}

fn escape_pdf(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii() && !character.is_ascii_control() {
                character
            } else {
                '?'
            }
        })
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize_json;

    #[test]
    fn clinician_exports_are_well_formed_and_non_diagnostic() {
        let events = normalize_json(
            r#"[{"occurred_at":"2026-01-02T12:00:00Z","event_type":"symptom","name":"Synthetic discomfort","severity":3}]"#,
        )
        .unwrap();
        let csv = clinician_csv(&events).unwrap();
        assert!(csv.contains("severity_0_to_10"));
        assert!(csv.contains("Synthetic discomfort"));
        let pdf = clinician_pdf(&events);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(String::from_utf8_lossy(&pdf).contains("No diagnosis, causal inference"));
    }
}
