use std::process::Command;

use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use crate::runtime::ContainerRuntime;

pub const DEFAULT_IMAGE: &str = "tfmttools-test-container:local";

const CONTAINERFILE: &str = "tests/container/Containerfile";

#[derive(Debug, Clone)]
pub struct ImageConfig {
    pub image: String,
    pub skip_build: bool,
    pub rebuild: bool,
}

impl ImageConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            image: env_or_default("TFMT_CONTAINER_IMAGE", DEFAULT_IMAGE),
            skip_build: env_flag("TFMT_CONTAINER_SKIP_BUILD")?,
            rebuild: env_flag("TFMT_CONTAINER_REBUILD")?,
        })
    }
}

pub fn ensure_image(
    runtime: &ContainerRuntime,
    config: &ImageConfig,
) -> Result<ImageBuild> {
    if config.skip_build {
        return Ok(ImageBuild::Skipped);
    }

    if !config.rebuild && image_exists(runtime, &config.image)? {
        return Ok(ImageBuild::AlreadyPresent);
    }

    build_image(runtime, &config.image)
}

fn image_exists(runtime: &ContainerRuntime, image: &str) -> Result<bool> {
    let output = Command::new(runtime.command())
        .args(["image", "inspect", image])
        .output()?;

    Ok(output.status.success())
}

fn build_image(runtime: &ContainerRuntime, image: &str) -> Result<ImageBuild> {
    let output = Command::new(runtime.command())
        .args(["build", "-f", CONTAINERFILE, "-t", image, "."])
        .output()?;

    if output.status.success() {
        Ok(ImageBuild::Built)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(eyre!(
            "failed to build container image {image} with {}: {stderr}",
            runtime.command()
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageBuild {
    AlreadyPresent,
    Built,
    Skipped,
}

impl ImageBuild {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyPresent => "already_present",
            Self::Built => "built",
            Self::Skipped => "skipped",
        }
    }
}

fn env_flag(name: &str) -> Result<bool> {
    match std::env::var(name) {
        Ok(value) if value == "1" => Ok(true),
        Ok(value) if value == "0" || value.is_empty() => Ok(false),
        Ok(value) => Err(eyre!("{name} must be 1 when set, got {value:?}")),
        Err(std::env::VarError::NotPresent) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub fn required_env_flag(name: &str) -> Result<bool> {
    env_flag(name)
}

pub fn required_env_u64(name: &str, default: u64) -> Result<u64> {
    let value = match std::env::var(name) {
        Ok(value) if value.is_empty() => return Ok(default),
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(default),
        Err(error) => return Err(error.into()),
    };

    let parsed = value.parse::<u64>().map_err(|_| {
        eyre!("{name} must be a positive integer, got {value:?}")
    })?;

    (parsed > 0)
        .then_some(parsed)
        .ok_or_eyre(format!("{name} must be greater than zero"))
}
