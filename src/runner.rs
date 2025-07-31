use std::time::Duration;

use jane_eyre::eyre;
use rand::{rng, seq::SliceRandom};
use reqwest::blocking::Client;
use serde_json::Value;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{actions::set_output_parameter, http::ClientExt};

#[derive(clap::Subcommand, Debug)]
#[allow(private_interfaces)]
pub enum RunnerCommand {
    Select(Select),
}

impl RunnerCommand {
    pub fn run(self) -> eyre::Result<()> {
        match self {
            RunnerCommand::Select(subcommand) => subcommand.run(),
        }
    }
}

/// Selects a self-hosted runner if available, or else a GitHub-hosted runner.
/// We generate a unique id for the workload, then ask our monitor API to
/// reserve a self-hosted runner for us.
#[derive(clap::Args, Debug)]
struct Select {
    /// `${{ github.repository }}`
    #[arg(long)]
    github_repository: String,
    /// `${{ github.run_id }}`
    #[arg(long)]
    github_run_id: String,
    /// `${{ secrets.SERVO_CI_MONITOR_API_TOKEN }}`
    #[arg(long)]
    monitor_api_token: String,
    /// e.g. ubuntu-22.04
    #[arg(long)]
    github_hosted_runner_label: String,
    /// e.g. servo-ubuntu2204
    #[arg(long)]
    self_hosted_image_name: String,
    #[arg(long)]
    force_github_hosted_runner: bool,
}

impl Select {
    pub fn run(self) -> eyre::Result<()> {
        let Self {
            github_repository,
            github_run_id: run_id,
            monitor_api_token,
            github_hosted_runner_label,
            self_hosted_image_name,
            force_github_hosted_runner,
        } = &self;

        let fall_back_to_github_hosted = || -> eyre::Result<()> {
            info!("Falling back to GitHub-hosted runner");
            set_output_parameter("selected_runner_label", github_hosted_runner_label)?;
            set_output_parameter("is_self_hosted", false)?;
            Ok(())
        };

        let unique_id = Uuid::new_v4();
        set_output_parameter("unique_id", unique_id.to_string())?;

        // TODO: this is an environment variable, not a configuration variable
        if std::env::var_os("NO_SELF_HOSTED_RUNNERS").is_some() {
            info!("NO_SELF_HOSTED_RUNNERS is set!");
            return fall_back_to_github_hosted();
        }

        if *force_github_hosted_runner {
            info!("--force-github-hosted-runner is set!");
            return fall_back_to_github_hosted();
        }

        let try_server = |base_url| -> eyre::Result<Value> {
            let client = Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(30))
                .build()?;
            let response = client
                .logged_post(format!("{base_url}/profile/{self_hosted_image_name}/take?unique_id={unique_id}&qualified_repo={github_repository}&run_id={run_id}"))?
                .bearer_auth(monitor_api_token)
                .send()?
                .error_for_status()?
                .json::<Value>()?;
            debug!(?response);
            Ok(response)
        };
        let mut server_base_urls = [
            "https://ci0.servo.org",
            "https://ci1.servo.org",
            "https://ci2.servo.org",
        ];
        server_base_urls.shuffle(&mut rng());
        for base_url in server_base_urls {
            match try_server(base_url) {
                Ok(response) if !response.is_null() => {
                    set_output_parameter(
                        "selected_runner_label",
                        format!("reserved-for:{unique_id}"),
                    )?;
                    set_output_parameter("is_self_hosted", true)?;
                    return Ok(());
                }
                other => warn!(?other),
            }
        }

        fall_back_to_github_hosted()
    }
}
