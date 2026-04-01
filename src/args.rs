use std::path::PathBuf;

use color_eyre::eyre::Result;
use pico_args::Arguments;

pub struct Args {
    pub current: PathBuf,
    pub up: PathBuf,
    pub proposed: PathBuf,
    pub down: PathBuf,
    pub parameters: PathBuf,
    pub connection_details: PathBuf,
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
        })
    }
}
