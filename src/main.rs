use std::path::PathBuf;

use anyhow::{Context, Result};
use health_event_ledger::{
    HealthStore, clinician_csv, clinician_pdf, daily_summaries, normalize_json,
};

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("import") => import(arguments.next(), arguments.next()),
        Some("summary") => summary(arguments.next()),
        Some("export-csv") => export(arguments.next(), arguments.next(), false),
        Some("export-pdf") => export(arguments.next(), arguments.next(), true),
        Some("demo") | None => demo(),
        Some(command) => anyhow::bail!("unknown command: {command}"),
    }
}

fn import(database: Option<String>, source: Option<String>) -> Result<()> {
    let database = database.context("database path is required")?;
    let source = source.context("JSON source path is required")?;
    let json = std::fs::read_to_string(source).context("read JSON source")?;
    let events = normalize_json(&json)?;
    let mut store = HealthStore::open(database)?;
    let inserted = store.import(&events)?;
    println!("normalized={} inserted={inserted}", events.len());
    Ok(())
}

fn summary(database: Option<String>) -> Result<()> {
    let database = database.context("database path is required")?;
    let store = HealthStore::open(database)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&daily_summaries(&store.timeline()?))?
    );
    Ok(())
}

fn export(database: Option<String>, output: Option<String>, pdf: bool) -> Result<()> {
    let database = database.context("database path is required")?;
    let output = PathBuf::from(output.context("output path is required")?);
    let store = HealthStore::open(database)?;
    let events = store.timeline()?;
    if pdf {
        std::fs::write(output, clinician_pdf(&events)).context("write PDF report")?;
    } else {
        std::fs::write(output, clinician_csv(&events)?).context("write CSV report")?;
    }
    println!("exported_events={}", events.len());
    Ok(())
}

fn demo() -> Result<()> {
    let events = normalize_json(include_str!("../examples/synthetic-events.json"))?;
    let mut store = HealthStore::in_memory()?;
    let inserted = store.import(&events)?;
    let timeline = store.timeline()?;
    let summaries = daily_summaries(&timeline);
    println!(
        "events={inserted} days={} csv_bytes={} pdf_bytes={} descriptive_only=true",
        summaries.len(),
        clinician_csv(&timeline)?.len(),
        clinician_pdf(&timeline).len()
    );
    Ok(())
}
