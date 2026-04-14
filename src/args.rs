use std::path::PathBuf;

use color_eyre::eyre::Result;
use pico_args::Arguments;

#[derive(Clone, Debug)]
pub struct Args {
    pub current: PathBuf,
    pub up: PathBuf,
    pub proposed: PathBuf,
    pub down: PathBuf,
    pub parameters: PathBuf,
    pub connection_details: PathBuf,
    pub settings: Settings,
}

#[derive(Copy, Clone, Debug)]
pub struct Settings {
    pub warmups: usize,
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
