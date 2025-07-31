mod actions;
mod github;
mod http;
mod runner;

use clap::Parser as _;
use cmd_lib::run_cmd;
use jane_eyre::eyre;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[derive(clap::Parser, Debug)]
pub enum Command {
    Hello,
    #[command(subcommand)]
    Runner(crate::runner::RunnerCommand),
}

fn main() -> eyre::Result<()> {
    jane_eyre::install()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(if std::env::var("RUST_LOG").is_ok() {
            tracing_subscriber::EnvFilter::builder().from_env_lossy()
        } else {
            "ci=info,cmd_lib=info".parse()?
        })
        .init();

    Command::parse().run()
}

impl Command {
    fn run(self) -> eyre::Result<()> {
        match self {
            Command::Hello => hello(),
            Command::Runner(subcommand) => subcommand.run(),
        }
    }
}

#[tracing::instrument]
fn hello() -> eyre::Result<()> {
    run_cmd!(echo hello world >&2)?;

    Ok(())
}
