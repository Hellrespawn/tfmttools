use std::ffi::OsStr;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::scenario::ScenarioMount;

#[derive(Debug, Clone)]
pub struct ContainerRuntime {
    command: String,
}

impl ContainerRuntime {
    pub fn detect(required: bool) -> Result<Option<Self>> {
        if let Some(command) = explicit_runtime()? {
            if command_exists(&command) {
                return Ok(Some(Self { command }));
            }

            return Err(eyre!(
                "TFMT_CONTAINER_RUNTIME names missing runtime {command:?}"
            ));
        }

        for command in ["docker", "podman"] {
            if command_exists(command) {
                return Ok(Some(Self { command: command.to_owned() }));
            }
        }

        if required {
            Err(eyre!("no container runtime found; tried docker and podman"))
        } else {
            Ok(None)
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn version_output(&self) -> Result<String> {
        let output = Command::new(self.command()).arg("--version").output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let text = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if output.status.success() && !text.is_empty() {
            Ok(text)
        } else {
            Err(eyre!(
                "failed to query runtime version from {}",
                self.command()
            ))
        }
    }

    pub fn create_volume(
        &self,
        name: &str,
        mount: &ScenarioMount,
    ) -> Result<()> {
        if let Some(host_bind_dir) = mount.host_bind_dir(name) {
            fs_err::create_dir_all(host_bind_dir)?;
        }

        let mut args = vec!["volume".to_owned(), "create".to_owned()];

        if let Some(driver) = mount.driver() {
            args.push("--driver".to_owned());
            args.push(driver.to_owned());
        }

        for (key, value) in mount.resolved_driver_opts(name) {
            args.push("--opt".to_owned());
            args.push(format!("{key}={value}"));
        }

        args.push(name.to_owned());

        let output = Command::new(self.command()).args(&args).output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(command_error(self.command(), &args, &output))
        }
    }

    pub fn remove_volume(&self, name: &str) -> Result<()> {
        let output = Command::new(self.command())
            .args(["volume", "rm", "-f", name])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(command_error(
                self.command(),
                &[
                    "volume".to_owned(),
                    "rm".to_owned(),
                    "-f".to_owned(),
                    name.to_owned(),
                ],
                &output,
            ))
        }
    }

    pub fn run_with_timeout<I, S>(
        &self,
        args: I,
        timeout_seconds: u64,
    ) -> Result<RuntimeCommandResult>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        let mut child = Command::new(self.command())
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let started = Instant::now();
        let timeout = Duration::from_secs(timeout_seconds);

        loop {
            if child.try_wait()?.is_some() {
                let output = child.wait_with_output()?;
                return Ok(RuntimeCommandResult {
                    arguments: args,
                    output,
                    duration_ms: started.elapsed().as_millis(),
                    timed_out: false,
                });
            }

            if started.elapsed() >= timeout {
                child.kill()?;
                let output = child.wait_with_output()?;
                return Ok(RuntimeCommandResult {
                    arguments: args,
                    output,
                    duration_ms: started.elapsed().as_millis(),
                    timed_out: true,
                });
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[derive(Debug)]
pub struct RuntimeCommandResult {
    pub arguments: Vec<String>,
    pub output: Output,
    pub duration_ms: u128,
    pub timed_out: bool,
}

fn explicit_runtime() -> Result<Option<String>> {
    match std::env::var("TFMT_CONTAINER_RUNTIME") {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn command_error(
    command: &str,
    args: &[String],
    output: &Output,
) -> color_eyre::Report {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let mut message =
        format!("runtime command failed: {command} {}", args.join(" "));

    if !stdout.is_empty() {
        message.push_str("\nstdout:\n");
        message.push_str(&stdout);
    }

    if !stderr.is_empty() {
        message.push_str("\nstderr:\n");
        message.push_str(&stderr);
    }

    eyre!(message)
}
