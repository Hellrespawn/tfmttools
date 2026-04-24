use std::process::Command;

use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use crate::runtime::ContainerRuntime;

pub const DEFAULT_IMAGE: &str = "tfmttools-test-container:local";

const CONTAINERFILE: &str = "tests/container/Containerfile";
const BUILD_CONTEXT: &str = ".";

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
) -> std::result::Result<ImageInfo, ImageBuildFailure> {
    let build = if config.skip_build {
        ImageBuild::Skipped
    } else if !config.rebuild
        && image_exists(runtime, &config.image).map_err(|error| {
            ImageBuildFailure::new(
                format!("failed to inspect container image {}: {error}", config.image),
                None,
                None,
            )
        })?
    {
        ImageBuild::AlreadyPresent
    } else {
        build_image(runtime, &config.image)?
    };

    Ok(ImageInfo {
        build,
        id: image_id(runtime, &config.image).map_err(|error| {
            ImageBuildFailure::new(
                format!("failed to inspect image id for {}: {error}", config.image),
                None,
                None,
            )
        })?,
        source: ImageSource::default(),
    })
}

fn image_exists(runtime: &ContainerRuntime, image: &str) -> Result<bool> {
    let output = Command::new(runtime.command())
        .args(["image", "inspect", image])
        .output()?;

    Ok(output.status.success())
}

fn build_image(
    runtime: &ContainerRuntime,
    image: &str,
) -> std::result::Result<ImageBuild, ImageBuildFailure> {
    let output = Command::new(runtime.command())
        .args(["build", "-f", CONTAINERFILE, "-t", image, BUILD_CONTEXT])
        .current_dir(workspace_root())
        .output()
        .map_err(|error| {
            ImageBuildFailure::new(
                format!("failed to run {} build for {image}: {error}", runtime.command()),
                None,
                None,
            )
        })?;

    if output.status.success() {
        Ok(ImageBuild::Built)
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(ImageBuildFailure::new(
            format!("failed to build container image {image} with {}", runtime.command()),
            (!stdout.is_empty()).then_some(stdout),
            (!stderr.is_empty()).then_some(stderr),
        ))
    }
}

fn workspace_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn image_id(runtime: &ContainerRuntime, image: &str) -> Result<Option<String>> {
    let output = Command::new(runtime.command())
        .args(["image", "inspect", "--format", "{{.Id}}", image])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!id.is_empty()).then_some(id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub build: ImageBuild,
    pub id: Option<String>,
    pub source: ImageSource,
}

#[derive(Debug, Clone)]
pub struct ImageBuildFailure {
    pub reason: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl ImageBuildFailure {
    fn new(
        reason: impl Into<String>,
        stdout: Option<String>,
        stderr: Option<String>,
    ) -> Self {
        Self { reason: reason.into(), stdout, stderr }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSource {
    pub build_context: String,
    pub containerfile: String,
    pub builder_base: String,
    pub runtime_base: String,
}

impl Default for ImageSource {
    fn default() -> Self {
        Self {
            build_context: BUILD_CONTEXT.to_owned(),
            containerfile: CONTAINERFILE.to_owned(),
            builder_base: "rust:1.89-bookworm".to_owned(),
            runtime_base: "debian:bookworm-slim".to_owned(),
        }
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
