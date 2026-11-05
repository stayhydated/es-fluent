use anyhow::{Context as _, Result, bail};
use es_fluent_shared::resource::INVENTORY_RUNNER_ENV;
use fs_err as fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(test)]
use std::process::Output;

/// Hides compiler warnings from runner builds without changing rustc arguments.
///
/// Setting `RUSTFLAGS` would replace configured `build.rustflags` and
/// `target.<triple>.rustflags` and give every unit a different compiler-cache
/// key than the application's own builds.
const CARGO_BUILD_WARNINGS_ENV: &str = "CARGO_BUILD_WARNINGS";
const RUSTFLAGS_ENV: &str = "RUSTFLAGS";
const CARGO_ENCODED_RUSTFLAGS_ENV: &str = "CARGO_ENCODED_RUSTFLAGS";

/// Configures `cmd` so runner builds do not fail or report on compiler warnings.
///
/// `CARGO_BUILD_WARNINGS=allow` only hides warnings. When the environment's
/// rustflags turn warnings into errors, as CI often does with
/// `RUSTFLAGS="-D warnings"`, `-A warnings` is appended to those flags. The
/// environment already replaces configured rustflags in that case, so
/// configured flags are unaffected.
fn allow_compiler_warnings(cmd: &mut Command) {
    cmd.env(CARGO_BUILD_WARNINGS_ENV, "allow");
    let encoded = env::var(CARGO_ENCODED_RUSTFLAGS_ENV).ok();
    let plain = env::var(RUSTFLAGS_ENV).ok();
    if let Some((name, value)) = rustflags_allowing_warnings(encoded.as_deref(), plain.as_deref()) {
        cmd.env(name, value);
    }
}

/// Returns the rustflags variable and value that undo a `-D warnings` in the
/// environment's effective rustflags, or `None` when warnings are not denied.
///
/// Cargo prefers `CARGO_ENCODED_RUSTFLAGS`, whose flags are separated by
/// `0x1f`, over whitespace-separated `RUSTFLAGS`.
fn rustflags_allowing_warnings(
    encoded: Option<&str>,
    plain: Option<&str>,
) -> Option<(&'static str, String)> {
    match (encoded, plain) {
        (Some(flags), _) => denies_warnings(flags.split('\x1f')).then(|| {
            (
                CARGO_ENCODED_RUSTFLAGS_ENV,
                format!("{flags}\x1f-A\x1fwarnings"),
            )
        }),
        (None, Some(flags)) => denies_warnings(flags.split_whitespace())
            .then(|| (RUSTFLAGS_ENV, format!("{flags} -A warnings"))),
        (None, None) => None,
    }
}

fn denies_warnings<'a>(flags: impl Iterator<Item = &'a str>) -> bool {
    let mut previous = "";
    for flag in flags {
        if matches!(flag, "-Dwarnings" | "--deny=warnings")
            || (matches!(previous, "-D" | "--deny") && flag == "warnings")
        {
            return true;
        }
        previous = flag;
    }
    false
}

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

    pub(super) fn cargo_command(&self) -> Command {
        let mut command = Command::new("cargo");
        command
            .current_dir(self.temp_dir)
            .env("CARGO_TARGET_DIR", self.temp_dir.join("target"));
        command
    }

    /// Write the Cargo.toml for the runner crate.
    pub(super) fn write_cargo_toml(&self, cargo_toml_content: &str) -> Result<()> {
        super::write_if_changed(&self.temp_dir.join("Cargo.toml"), cargo_toml_content)
            .context("Failed to write .es-fluent/Cargo.toml")
    }

    /// Write the .cargo/config.toml for the runner crate.
    pub(super) fn write_cargo_config(&self, config_content: &str) -> Result<()> {
        let cargo_dir = self.temp_dir.join(".cargo");
        fs::create_dir_all(&cargo_dir).context("Failed to create .es-fluent/.cargo directory")?;
        super::write_if_changed(&cargo_dir.join("config.toml"), config_content)
            .context("Failed to write .es-fluent/.cargo/config.toml")
    }

    /// Run `cargo run` on the runner crate.
    pub(super) fn run_cargo(&self, bin_name: Option<&str>, args: &[String]) -> Result<String> {
        let mut cmd = self.cargo_command();
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--")
            .args(args)
            .env(INVENTORY_RUNNER_ENV, "1");
        allow_compiler_warnings(&mut cmd);

        if env::var("NO_COLOR").is_err() {
            cmd.env("CLICOLOR_FORCE", "1");
        }

        let output = cmd.output().context("Failed to run cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo run failed: {}", stderr)
        }

        Ok(String::from_utf8_lossy_owned(output.stdout))
    }

    /// Run `cargo run` on the runner crate and capture output.
    #[cfg(test)]
    pub(super) fn run_cargo_with_output(
        &self,
        bin_name: Option<&str>,
        args: &[String],
    ) -> Result<Output> {
        let mut cmd = self.cargo_command();
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--")
            .args(args)
            .env(INVENTORY_RUNNER_ENV, "1");
        allow_compiler_warnings(&mut cmd);

        let output = cmd.output().context("Failed to run cargo")?;

        if output.status.success() {
            Ok(output)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo run failed: {}", stderr)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustflags_are_left_alone_unless_they_deny_warnings() {
        assert_eq!(rustflags_allowing_warnings(None, None), None);
        assert_eq!(
            rustflags_allowing_warnings(None, Some("-C target-cpu=native")),
            None
        );
        assert_eq!(
            rustflags_allowing_warnings(Some("--cfg\x1fci"), Some("-D warnings")),
            None,
            "CARGO_ENCODED_RUSTFLAGS takes precedence over RUSTFLAGS"
        );
    }

    #[test]
    fn denied_warnings_are_allowed_in_the_effective_rustflags() {
        for flags in [
            "-D warnings",
            "-Dwarnings",
            "--deny warnings",
            "--deny=warnings",
        ] {
            assert_eq!(
                rustflags_allowing_warnings(None, Some(flags)),
                Some((RUSTFLAGS_ENV, format!("{flags} -A warnings"))),
                "{flags}"
            );
        }
        assert_eq!(
            rustflags_allowing_warnings(Some("-D\x1fwarnings"), None),
            Some((
                CARGO_ENCODED_RUSTFLAGS_ENV,
                "-D\x1fwarnings\x1f-A\x1fwarnings".to_string()
            ))
        );
    }
}
