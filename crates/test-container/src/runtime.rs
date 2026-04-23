use std::process::Command;

use color_eyre::Result;
use color_eyre::eyre::eyre;

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
