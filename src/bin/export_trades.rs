//! Export closed trades and daily stats from SQLite persistence.
//!
//! Usage:
//!   cargo run --bin export-trades -- --format csv --output closed_trades.csv
//!   cargo run --bin export-trades -- --format json --output closed_trades.json
//!   cargo run --bin export-trades -- --daily-stats --output daily_stats.csv

use palm_oil_bot::modules::trading::PositionDatabase;
use std::env;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut format = "csv".to_string();
    let mut output = None;
    let mut daily_stats = false;
    let mut db_path = env::var("PERSISTENCE_DB_PATH").unwrap_or_else(|_| "data/positions.db".to_string());

    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format" => {
                if let Some(val) = args.get(idx + 1) {
                    format = val.clone();
                    idx += 1;
                }
            }
            "--output" => {
                if let Some(val) = args.get(idx + 1) {
                    output = Some(val.clone());
                    idx += 1;
                }
            }
            "--daily-stats" => {
                daily_stats = true;
            }
            "--db" => {
                if let Some(val) = args.get(idx + 1) {
                    db_path = val.clone();
                    idx += 1;
                }
            }
            _ => {}
        }
        idx += 1;
    }

    let output_path = output.unwrap_or_else(|| {
        if daily_stats {
            "daily_stats.csv".to_string()
        } else if format == "json" {
            "closed_trades.json".to_string()
        } else {
            "closed_trades.csv".to_string()
        }
    });

    let db = PositionDatabase::new(&db_path)?;
    let path = PathBuf::from(output_path);

    if daily_stats {
        db.export_daily_stats_csv(&path)?;
        println!("Exported daily stats to {}", path.display());
        return Ok(());
    }

    if format == "json" {
        db.export_closed_trades_json(&path)?;
        println!("Exported closed trades JSON to {}", path.display());
    } else {
        db.export_closed_trades_csv(&path)?;
        println!("Exported closed trades CSV to {}", path.display());
    }

    Ok(())
}
