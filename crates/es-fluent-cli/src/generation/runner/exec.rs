use anyhow::{Context as _, Result, bail};
use fs_err as fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(test)]
use std::process::Output;

pub(super) struct RunnerCrate<'a> {
    temp_dir: &'a Path,
}

impl RunnerCrate<'_> {
    pub(super) fn new(temp_dir: &Path) -> RunnerCrate<'_> {
        RunnerCrate { temp_dir }
    }

    pub(super) fn manifest_path(&self) -> PathBuf {
        self.temp_dir.join("Cargo.toml")
    }

    /// Write the Cargo.toml for the runner crate.
    pub(super) fn write_cargo_toml(&self, cargo_toml_content: &str) -> Result<()> {
        fs::write(self.temp_dir.join("Cargo.toml"), cargo_toml_content)
            .context("Failed to write .es-fluent/Cargo.toml")
    }

    /// Write the .cargo/config.toml for the runner crate.
    pub(super) fn write_cargo_config(&self, config_content: &str) -> Result<()> {
        let cargo_dir = self.temp_dir.join(".cargo");
        fs::create_dir_all(&cargo_dir).context("Failed to create .es-fluent/.cargo directory")?;
        fs::write(cargo_dir.join("config.toml"), config_content)
            .context("Failed to write .es-fluent/.cargo/config.toml")
    }

    /// Run `cargo run` on the runner crate.
    pub(super) fn run_cargo(&self, bin_name: Option<&str>, args: &[String]) -> Result<String> {
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--")
            .args(args)
            .current_dir(self.temp_dir)
            .env("RUSTFLAGS", runner_rustflags());

        if env::var("NO_COLOR").is_err() {
            cmd.env("CLICOLOR_FORCE", "1");
        }

        let output = cmd.output().context("Failed to run cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo run failed: {}", stderr)
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run `cargo run` on the runner crate and capture output.
    #[cfg(test)]
    pub(super) fn run_cargo_with_output(
        &self,
        bin_name: Option<&str>,
        args: &[String],
    ) -> Result<Output> {
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--")
            .args(args)
            .current_dir(self.temp_dir)
            .env("RUSTFLAGS", runner_rustflags());

        let output = cmd.output().context("Failed to run cargo")?;

        if output.status.success() {
            Ok(output)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo run failed: {}", stderr)
        }
    }
}

fn runner_rustflags() -> String {
    match env::var("RUSTFLAGS") {
        Ok(flags) if !flags.trim().is_empty() => format!("{flags} -A warnings"),
        _ => "-A warnings".to_string(),
    }
}

/// Run `cargo run` on the runner crate.
#[cfg(test)]
pub fn run_cargo(temp_dir: &Path, bin_name: Option<&str>, args: &[String]) -> Result<String> {
    RunnerCrate::new(temp_dir).run_cargo(bin_name, args)
}

/// Run `cargo run` on the runner crate and capture output.
#[cfg(test)]
pub fn run_cargo_with_output(
    temp_dir: &Path,
    bin_name: Option<&str>,
    args: &[String],
) -> Result<Output> {
    RunnerCrate::new(temp_dir).run_cargo_with_output(bin_name, args)
}
