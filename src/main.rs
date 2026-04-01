use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use color_eyre::eyre::{eyre, Result};
use serde::Deserialize;
use sqlx::{types::Json, PgPool, Row};
use uuid::Uuid;

mod args;

use crate::args::Args;

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
    sqlx::query(query).execute(&mut *conn).await?;
    conn.close().await?;

    Ok(())
}

async fn benchmark_query(pool: &PgPool, query: &str, parameters: &[Uuid]) -> Result<()> {
    let analyse_query = format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", query);

    for parameter in parameters {
        let mut tx = pool.begin().await?;
        let now = Instant::now();

        let result = sqlx::query(&analyse_query)
            .bind(parameter)
            .fetch_one(&mut *tx)
            .await?;

        let plans: Json<Vec<QueryPlan>> = result.try_get(0)?;
        let plan = &plans[0];

        let elapsed = now.elapsed();
        tx.rollback().await?;

        println!(
            "Parameter {}: Query took {} ms, planning time: {} ms, execution time: {} ms",
            parameter,
            elapsed.as_millis(),
            plan.planning_time,
            plan.execution_time
        );
    }

    Ok(())
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

    benchmark_query(&pool, &current_query, &parameters).await?;
    execute_as_owner(&pool, &up_query).await?;

    benchmark_query(&pool, &proposed_query, &parameters).await?;
    execute_as_owner(&pool, &down_query).await?;

    Ok(())
}
