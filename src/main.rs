use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};

use color_eyre::eyre::{eyre, Result};
use indicatif::ProgressIterator;
use owo_colors::OwoColorize;
use serde::Deserialize;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use tabled::{Table, Tabled};
use uuid::Uuid;

mod args;

use crate::args::{Args, Settings};

#[derive(Deserialize)]
struct QueryPlan {
    #[serde(rename = "Planning Time")]
    planning_time: f64,
    #[serde(rename = "Execution Time")]
    execution_time: f64,
}

fn read_file_content(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read file {}: {}", path.display(), e))?;

    Ok(content)
}

async fn execute_as_owner(pool: &PgPool, query: &str) -> Result<()> {
    let mut conn = pool.acquire().await?;
    sqlx::query("SET ROLE TO owner").execute(&mut *conn).await?;

    for statement in query.split(';') {
        let trimmed = statement.trim();

        if !trimmed.is_empty() {
            println!("  Executing: {}", trimmed);
            sqlx::query(trimmed).execute(&mut *conn).await?;
        }
    }

    conn.close().await?;

    Ok(())
}

struct BenchmarkOutcome {
    elapsed: Duration,
    planning_time: f64,
    execution_time: f64,
}

async fn run_query(pool: &PgPool, query: &str, parameter: Uuid) -> Result<BenchmarkOutcome> {
    let mut tx = pool.begin().await?;
    let now = Instant::now();

    let result = sqlx::query(&query)
        .bind(parameter)
        .fetch_one(&mut *tx)
        .await?;

    let plans: Json<Vec<QueryPlan>> = result.try_get(0)?;
    let plan = &plans[0];

    let elapsed = now.elapsed();
    tx.rollback().await?;

    Ok(BenchmarkOutcome {
        elapsed,
        planning_time: plan.planning_time,
        execution_time: plan.execution_time,
    })
}

async fn benchmark_query(
    pool: &PgPool,
    query: &str,
    parameters: &[Uuid],
    settings: Settings,
) -> Result<HashMap<Uuid, BenchmarkOutcome>> {
    let analyse_query = format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", query);
    let mut results = HashMap::new();

    for parameter in parameters {
        println!("Benchmarking parameter {}...", parameter);
        println!("    Running {} warmup runs...", settings.warmups);

        for _ in (0..settings.warmups).progress() {
            run_query(pool, &analyse_query, *parameter).await?;
        }

        let mut outcomes = Vec::new();

        println!("    Running {} benchmark runs...", settings.runs);

        for _ in (0..settings.runs).progress() {
            let outcome = run_query(pool, &analyse_query, *parameter).await?;
            outcomes.push(outcome);
        }

        // remove the fastest and slowest runs to mitigate outliers
        outcomes.sort_by_key(|o| o.elapsed);
        outcomes.pop();
        outcomes.remove(0);

        // summarise the results by taking the average of the remaining runs
        let average_elapsed =
            outcomes.iter().map(|o| o.elapsed).sum::<Duration>() / (outcomes.len() as u32);
        let average_planning_time =
            outcomes.iter().map(|o| o.planning_time).sum::<f64>() / (outcomes.len() as f64);
        let average_execution_time =
            outcomes.iter().map(|o| o.execution_time).sum::<f64>() / (outcomes.len() as f64);

        results.insert(
            *parameter,
            BenchmarkOutcome {
                elapsed: average_elapsed,
                planning_time: average_planning_time,
                execution_time: average_execution_time,
            },
        );
    }

    Ok(results)
}

fn truncate_uuid(id: &Uuid) -> String {
    let s = id.to_string();
    format!("{}...{}", &s[..4], &s[s.len() - 4..])
}

fn format_delta(delta: f64) -> String {
    let s = format!("{:+.2}% {}", delta, if delta < 0.0 { "↓" } else { "↑" });
    if delta < 0.0 {
        s.green().to_string()
    } else {
        s.red().to_string()
    }
}

#[derive(Tabled)]
struct ResultRow {
    #[tabled(rename = "UUID")]
    uuid: String,
    #[tabled(rename = "Cur exec")]
    cur_exec: String,
    #[tabled(rename = "Prop exec")]
    prop_exec: String,
    #[tabled(rename = "Δ exec")]
    delta_exec: String,
    #[tabled(rename = "Cur plan")]
    cur_plan: String,
    #[tabled(rename = "Prop plan")]
    prop_plan: String,
    #[tabled(rename = "Δ plan")]
    delta_plan: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse()?;

    let current_query = read_file_content(&args.current)?;
    let up_query = read_file_content(&args.up)?;
    let proposed_query = read_file_content(&args.proposed)?;
    let down_query = read_file_content(&args.down)?;
    let parameters = read_file_content(&args.parameters)?;
    let connection_details = read_file_content(&args.connection_details)?;

    let parameters = parameters
        .lines()
        .filter_map(|line| Uuid::from_str(line).ok())
        .collect::<Vec<_>>();

    // open a connection to the database
    let pool = PgPool::connect(&connection_details).await?;

    println!("== Current query ==");
    let current = benchmark_query(&pool, &current_query, &parameters, args.settings).await?;

    println!("\n== Applying schema changes ==");
    execute_as_owner(&pool, &up_query).await?;

    println!("\n== Proposed query ==");
    let proposed = benchmark_query(&pool, &proposed_query, &parameters, args.settings).await?;

    println!("\n== Rolling back schema changes ==");
    execute_as_owner(&pool, &down_query).await?;

    // build results table
    let mut rows = Vec::new();
    let mut exec_deltas: Vec<f64> = Vec::new();
    let mut plan_deltas: Vec<f64> = Vec::new();

    for parameter in &parameters {
        let current_outcome = current.get(parameter).unwrap();
        let proposed_outcome = proposed.get(parameter).unwrap();

        let exec_delta = (proposed_outcome.execution_time - current_outcome.execution_time)
            / current_outcome.execution_time
            * 100.0;
        let plan_delta = (proposed_outcome.planning_time - current_outcome.planning_time)
            / current_outcome.planning_time
            * 100.0;

        exec_deltas.push(exec_delta);
        plan_deltas.push(plan_delta);

        rows.push(ResultRow {
            uuid: truncate_uuid(parameter),
            cur_exec: format!("{:.2}ms", current_outcome.execution_time),
            prop_exec: format!("{:.2}ms", proposed_outcome.execution_time),
            delta_exec: format_delta(exec_delta),
            cur_plan: format!("{:.2}ms", current_outcome.planning_time),
            prop_plan: format!("{:.2}ms", proposed_outcome.planning_time),
            delta_plan: format_delta(plan_delta),
        });
    }

    println!("\n== Results ==");
    println!("{}", Table::new(rows));

    let n = exec_deltas.len() as f64;
    let total = exec_deltas.len();

    let avg_exec = exec_deltas.iter().sum::<f64>() / n;
    let min_exec = exec_deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_exec = exec_deltas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let improved_exec = exec_deltas.iter().filter(|&&d| d < 0.0).count();

    let avg_plan = plan_deltas.iter().sum::<f64>() / n;
    let min_plan = plan_deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_plan = plan_deltas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let improved_plan = plan_deltas.iter().filter(|&&d| d < 0.0).count();

    println!(
        "Execution time:  avg {}   min {}   max {}   improved {}/{}",
        format_delta(avg_exec),
        format_delta(min_exec),
        format_delta(max_exec),
        improved_exec,
        total,
    );
    println!(
        "Planning time:   avg {}   min {}   max {}   improved {}/{}",
        format_delta(avg_plan),
        format_delta(min_plan),
        format_delta(max_plan),
        improved_plan,
        total,
    );

    Ok(())
}
