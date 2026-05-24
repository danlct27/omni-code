use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::AppConfig;

pub fn run_stats(config: &AppConfig) {
    let db_path = db_path();
    if !db_path.exists() {
        println!("No logs found. Run the proxy first.");
        return;
    }

    let conn = Connection::open(&db_path).expect("failed to open logs.db");

    // Build pricing map from config
    let pricing: HashMap<String, (f64, f64)> = config
        .pricing
        .iter()
        .map(|p| (p.model.clone(), (p.input_per_m, p.output_per_m)))
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT date(timestamp, 'unixepoch') as day, model, 
                    COUNT(*) as reqs, SUM(tokens_in) as t_in, SUM(tokens_out) as t_out
             FROM requests
             GROUP BY day, model
             ORDER BY day DESC, model",
        )
        .expect("query failed");

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .expect("query failed");

    println!(
        "{:<12} {:<20} {:>6} {:>10} {:>10} {:>10}",
        "Date", "Model", "Reqs", "Tokens In", "Tokens Out", "Cost ($)"
    );
    println!("{}", "-".repeat(72));

    for row in rows {
        let (day, model, reqs, t_in, t_out) = row.unwrap();
        let (in_price, out_price) = pricing
            .iter()
            .find(|(k, _)| model.starts_with(k.as_str()))
            .map(|(_, v)| *v)
            .unwrap_or((0.0, 0.0));

        let cost = (t_in as f64 * in_price + t_out as f64 * out_price) / 1_000_000.0;

        println!(
            "{:<12} {:<20} {:>6} {:>10} {:>10} {:>10.4}",
            day, model, reqs, t_in, t_out, cost
        );
    }
}

fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".omni-code/logs.db")
}
