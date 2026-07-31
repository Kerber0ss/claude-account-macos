use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::AppPaths;
use crate::state::{self, Authentication};

const AUTH_ENVIRONMENT: [&str; 5] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_MODEL",
    "CLAUDE_CODE_OAUTH_TOKEN",
];
const API_CONFIG_FILE: &str = "api.json";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfig {
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

pub fn exec_active_profile(paths: &AppPaths, arguments: &[OsString]) -> Result<()> {
    let state = state::load(paths)?;
    let active = state
        .active
        .as_deref()
        .context("no active profile; run `claude account add NAME` or `claude account use NAME`")?;
    let profile = state
        .profiles
        .get(active)
        .with_context(|| format!("active profile `{active}` does not exist"))?;
    let real_claude = state
        .real_claude
        .as_deref()
        .context("real Claude executable is not configured; run `claude-account install`")?;
    validate_executable(real_claude)?;

    let mut command = match profile.authentication {
        Authentication::OAuth => managed_command(real_claude, &profile.config_dir),
        Authentication::Api => managed_api_command(real_claude, &profile.config_dir)?,
    };
    command.args(arguments);
    let error = command.exec();
    Err(error).with_context(|| format!("failed to execute {}", real_claude.display()))
}

pub fn ensure_api_config(config_dir: &Path) -> Result<PathBuf> {
    let path = api_config_path(config_dir);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .with_context(|| format!("failed to protect {}", path.display()))?;
        }
        Ok(_) => bail!(
            "refusing to use non-file API configuration {}",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&path)
                .with_context(|| format!("failed to create {}", path.display()))?;
            serde_json::to_writer_pretty(&mut file, &ApiConfig::default())
                .context("failed to write API configuration template")?;
            file.write_all(b"\n")
                .context("failed to finish API configuration template")?;
            file.sync_all()
                .context("failed to sync API configuration template")?;
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()))
        }
    }
    Ok(path)
}

pub fn remove_api_config(config_dir: &Path) -> Result<()> {
    let path = api_config_path(config_dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}

fn managed_api_command(real_claude: &Path, config_dir: &Path) -> Result<Command> {
    let config_path = api_config_path(config_dir);
    let config: ApiConfig = serde_json::from_slice(
        &fs::read(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let api_key = config.api_key.trim();
    if api_key.is_empty() {
        bail!("API key is empty; edit {}", config_path.display());
    }

    let mut command = managed_command(real_claude, config_dir);
    command.env("ANTHROPIC_API_KEY", api_key);
    if !config.base_url.trim().is_empty() {
        command.env("ANTHROPIC_BASE_URL", config.base_url.trim());
    }
    if !config.model.trim().is_empty() {
        command.env("ANTHROPIC_MODEL", config.model.trim());
    }
    Ok(command)
}

pub fn api_config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(API_CONFIG_FILE)
}

pub fn managed_command(real_claude: &Path, config_dir: &Path) -> Command {
    let mut command = Command::new(real_claude);
    command.env("CLAUDE_CONFIG_DIR", config_dir);

    if env::var_os("CLAUDE_ACCOUNT_PRESERVE_AUTH_ENV").as_deref() != Some("1".as_ref()) {
        for variable in AUTH_ENVIRONMENT {
            command.env_remove(variable);
        }
    }
    command
}

pub fn resolve_real_claude(
    configured: Option<&Path>,
    current_executable: &Path,
    paths: &AppPaths,
) -> Result<PathBuf> {
    if let Some(explicit) = env::var_os("CLAUDE_ACCOUNT_REAL_CLAUDE") {
        let explicit = PathBuf::from(explicit);
        validate_distinct_executable(&explicit, current_executable)?;
        return Ok(explicit);
    }

    if let Some(configured) = configured {
        if validate_distinct_executable(configured, current_executable).is_ok() {
            return Ok(configured.to_path_buf());
        }
    }

    let path = env::var_os("PATH").context("PATH is not set")?;
    for directory in env::split_paths(&path) {
        let candidate = if directory.as_os_str().is_empty() {
            env::current_dir()?.join("claude")
        } else {
            directory.join("claude")
        };
        if candidate == paths.shim {
            continue;
        }
        if validate_distinct_executable(&candidate, current_executable).is_ok() {
            return Ok(candidate);
        }
    }

    bail!(
        "could not find the real `claude` executable; pass it with \
         `claude-account install --real /path/to/claude`"
    )
}

pub fn validate_executable(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Claude executable does not exist: {}", path.display()))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        bail!("path is not executable: {}", path.display());
    }
    Ok(())
}

fn validate_distinct_executable(candidate: &Path, current_executable: &Path) -> Result<()> {
    validate_executable(candidate)?;
    let candidate_canonical = fs::canonicalize(candidate)
        .with_context(|| format!("failed to resolve {}", candidate.display()))?;
    let current_canonical = fs::canonicalize(current_executable)
        .with_context(|| format!("failed to resolve {}", current_executable.display()))?;
    if candidate_canonical == current_canonical {
        bail!("candidate points back to the claude-account wrapper");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_profile_passes_connection_settings_to_claude() {
        let temp = tempfile::tempdir().unwrap();
        let fake_claude = temp.path().join("claude");
        fs::write(
            &fake_claude,
            "#!/bin/sh\nprintf '%s|%s|%s' \"$ANTHROPIC_API_KEY\" \"$ANTHROPIC_BASE_URL\" \"$ANTHROPIC_MODEL\"\n",
        )
        .unwrap();
        fs::set_permissions(&fake_claude, fs::Permissions::from_mode(0o755)).unwrap();

        let config_dir = temp.path().join("profile");
        fs::create_dir(&config_dir).unwrap();
        let config_path = ensure_api_config(&config_dir).unwrap();
        fs::write(
            config_path,
            r#"{"apiKey":"gateway-key","baseUrl":"https://gateway.example/v1","model":"custom-model"}"#,
        )
        .unwrap();

        let output = managed_api_command(&fake_claude, &config_dir)
            .unwrap()
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "gateway-key|https://gateway.example/v1|custom-model"
        );
    }
}
