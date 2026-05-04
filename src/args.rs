use std::path::PathBuf;

use color_eyre::eyre::Result;
use pico_args::Arguments;

#[derive(Clone, Debug)]
pub struct Args {
    /// Path to a file which contains the current query.
    pub current: PathBuf,
    /// Path to a migration file that applies any required changes to the database schema for the proposed query.
    pub up: PathBuf,
    /// Path to a file which contains the proposed query.
    pub proposed: PathBuf,
    /// Path to a migration file that reverts the changes applied by the `up` migration.
    pub down: PathBuf,
    /// Path to a file which contains the parameters for the queries.
    pub parameters: PathBuf,
    /// Path to a file which contains the connection details for the database.
    pub connection_details: PathBuf,
    /// Settings for the benchmark, such as the number of warmup runs and the number of actual runs.
    pub settings: Settings,
}

#[derive(Copy, Clone, Debug)]
pub struct Settings {
    /// The number of warmup runs to perform before the actual benchmark runs.
    pub warmups: usize,
    /// The number of actual benchmark runs to perform.
    pub runs: usize,
}

impl Args {
    pub fn parse() -> Result<Self> {
        let mut args = Arguments::from_env();

        Ok(Self {
            current: args.value_from_str("--current")?,
            up: args.value_from_str("--up")?,
            proposed: args.value_from_str("--proposed")?,
            down: args.value_from_str("--down")?,
            parameters: args.value_from_str("--parameters")?,
            connection_details: args.value_from_str("--connection-details")?,
            settings: Settings {
                warmups: args.opt_value_from_str("--warmups")?.unwrap_or(5),
                runs: args.opt_value_from_str("--runs")?.unwrap_or(10),
            },
        })
    }
}
