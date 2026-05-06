use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};

use color_eyre::eyre::{eyre, Result};
use serde::Deserialize;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
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
        println!("Running {} warmup runs...", settings.warmups);

        for _ in 0..settings.warmups {
            run_query(pool, &analyse_query, *parameter).await?;
        }

        let mut outcomes = Vec::new();

        println!("Running {} benchmark runs...", settings.runs);

        for _ in 0..settings.runs {
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

    let current = benchmark_query(&pool, &current_query, &parameters, args.settings).await?;
    execute_as_owner(&pool, &up_query).await?;

    let proposed = benchmark_query(&pool, &proposed_query, &parameters, args.settings).await?;
    execute_as_owner(&pool, &down_query).await?;

    // compare the average execution and planning times based on percentage improvement or regression
    let mut deltas = HashMap::new();

    for parameter in &parameters {
        let current_outcome = current.get(parameter).unwrap();
        let proposed_outcome = proposed.get(parameter).unwrap();

        let execution_time_change = (proposed_outcome.execution_time
            - current_outcome.execution_time)
            / current_outcome.execution_time
            * 100.0;
        let planning_time_change = (proposed_outcome.planning_time - current_outcome.planning_time)
            / current_outcome.planning_time
            * 100.0;

        deltas.insert(*parameter, (execution_time_change, planning_time_change));

        println!(
            "Parameter {}: current execution time: {:.2}ms, current planning time: {:.2}ms, proposed execution time: {:.2}ms, proposed planning time: {:.2}ms, execution time change: {:.2}%, planning time change: {:.2}%",
            parameter,
            current_outcome.execution_time,
            current_outcome.planning_time,
            proposed_outcome.execution_time,
            proposed_outcome.planning_time,
            execution_time_change,
            planning_time_change
        );
    }

    // summarise the results by calculating the average percentage change across all parameters
    let average_execution_time_change = deltas
        .values()
        .map(|(execution_time_change, _)| *execution_time_change)
        .sum::<f64>()
        / (deltas.len() as f64);
    let average_planning_time_change = deltas
        .values()
        .map(|(_, planning_time_change)| *planning_time_change)
        .sum::<f64>()
        / (deltas.len() as f64);

    println!(
        "Average execution time change: {:.2}%, Average planning time change: {:.2}%",
        average_execution_time_change, average_planning_time_change
    );

    Ok(())
}
