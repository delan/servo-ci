use std::{collections::BTreeMap, thread::sleep, time::Duration};

use jane_eyre::eyre::{self, OptionExt};
use rand::{rng, seq::SliceRandom};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{actions::set_output_parameter, github::GithubApi, http::ClientExt};

#[derive(clap::Subcommand, Debug)]
#[allow(private_interfaces)]
pub enum RunnerCommand {
    Select(Select),
    Timeout(Timeout),
}

impl RunnerCommand {
    pub fn run(self) -> eyre::Result<()> {
        match self {
            RunnerCommand::Select(subcommand) => subcommand.run(),
            RunnerCommand::Timeout(subcommand) => subcommand.run(),
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

/// In the unlikely event a self-hosted runner was selected and reserved but it
/// goes down before the workload starts, cancel the workflow run.
#[derive(clap::Args, Debug)]
struct Timeout {
    /// (seconds)
    #[arg(long)]
    wait_time: u64,
    /// Unique id that allows the workload job to find the runner
    /// we are reserving for it (via runner labels), and allows the timeout
    /// job to find the workload job run (via the job’s friendly name), even
    /// if there are multiple instances in the workflow call tree.
    unique_id: String,
    /// `${{ github.repository }}`
    #[arg(long)]
    github_repository: String,
    /// `${{ github.run_id }}`
    #[arg(long)]
    github_run_id: String,
    /// `${{ secrets.GITHUB_TOKEN }}`
    #[arg(long)]
    github_token: String,
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

impl Timeout {
    fn run(self) -> eyre::Result<()> {
        let Self {
            wait_time,
            unique_id,
            github_repository,
            github_run_id,
            github_token,
        } = &self;
        let client = GithubApi::client(github_token)?;

        // Wait a bit
        sleep(Duration::from_secs(*wait_time));

        // Cancel if workload job is still queued
        let run_url = format!("/repos/{github_repository}/actions/runs/{github_run_id}");
        let response = client
            .get(format!("{run_url}/jobs"))?
            .send()?
            .error_for_status()?
            .json::<RunResponse>()?;
        let job = response
            .jobs
            .iter()
            .find(|job| job.name.contains(&format!("[{unique_id}]")))
            .ok_or_eyre("Job not found")?;
        if job.status == "queued" {
            error!("Timeout waiting for runner assignment!");
            error!("Hint: does this repo have permission to access the runner group?");
            error!("Hint: https://github.com/organizations/servo/settings/actions/runner-groups");
            info!("");
            info!("Cancelling workflow run");
            client
                .post(format!("{run_url}/cancel"))?
                .send()?
                .error_for_status()?;
        }

        Ok(())
    }
}

/// <https://docs.github.com/en/rest/actions/workflow-runs?apiVersion=2022-11-28#get-a-workflow-run>
#[derive(Debug, Deserialize)]
struct RunResponse {
    jobs: Vec<Job>,
}
#[derive(Debug, Deserialize)]
struct Job {
    name: String,
    status: String,
    #[serde(flatten)]
    _rest: BTreeMap<String, Value>,
}
