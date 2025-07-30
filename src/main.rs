use clap::Parser as _;
use cmd_lib::run_cmd;
use jane_eyre::eyre;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[derive(clap::Parser, Debug)]
pub enum Command {
    Hello,
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

    let command = Command::parse();
    match command {
        Command::Hello => run_cmd!(echo hello world >&2)?,
    }

    Ok(())
}
