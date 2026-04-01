use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use color_eyre::eyre::{eyre, Result};
use sqlx::PgPool;
use uuid::Uuid;

mod args;

use crate::args::Args;

fn read_file_content(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| eyre!("Failed to read file {}: {}", path.display(), e))?;

    Ok(content)
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

    // open a connection to the database
    let pool = PgPool::connect(&connection_details).await?;

    for parameter in parameters
        .lines()
        .filter_map(|line| Uuid::from_str(line).ok())
    {
        let mut tx = pool.begin().await?;
        let now = Instant::now();

        sqlx::query(&current_query)
            .bind(parameter)
            .execute(&mut *tx)
            .await?;

        let elapsed = now.elapsed();
        tx.rollback().await?;

        println!(
            "Parameter {}: Current query took {} ms",
            parameter,
            elapsed.as_millis()
        );
    }

    let mut conn = pool.acquire().await?;
    sqlx::query("SET ROLE TO owner").execute(&mut *conn).await?;
    sqlx::query(&up_query).execute(&mut *conn).await?;
    conn.close().await?;

    for parameter in parameters
        .lines()
        .filter_map(|line| Uuid::from_str(line).ok())
    {
        let mut tx = pool.begin().await?;
        let now = Instant::now();

        sqlx::query(&proposed_query)
            .bind(parameter)
            .execute(&mut *tx)
            .await?;

        let elapsed = now.elapsed();
        tx.rollback().await?;

        println!(
            "Parameter {}: Proposed query took {} ms",
            parameter,
            elapsed.as_millis()
        );
    }

    let mut conn = pool.acquire().await?;
    sqlx::query("SET ROLE TO owner").execute(&mut *conn).await?;
    sqlx::query(&down_query).execute(&mut *conn).await?;
    conn.close().await?;

    Ok(())
}
