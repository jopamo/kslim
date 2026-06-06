//! CLI startup entrypoint.

use clap::Parser;

use super::Cli;

pub(crate) fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if let Err(e) = crate::commands::run(cli) {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}
