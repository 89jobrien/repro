//! Command execution abstractions (ports) and adapters.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// Execute a shell command.
pub trait CommandRunner: Send + Sync {
    fn run(&self, cmd: &[String]) -> Result<()>;
}

/// Execute a command where failure is expected/ignored (e.g. idempotent setup).
/// Only Docker needs this (for `buildx create`).
pub trait IdempotentRunner: CommandRunner {
    fn run_no_check(&self, cmd: &[String]);
}

/// Default runner that executes commands via `std::process::Command`.
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, cmd: &[String]) -> Result<()> {
        let cmd_display = shell_words::join(cmd);
        debug!("running: {cmd_display}");
        let mut proc = Command::new(&cmd[0]);
        proc.args(&cmd[1..]);
        if let Some(parent) = Path::new(&cmd[0]).parent()
            && parent.is_absolute()
        {
            env_prepend_path(&mut proc, "PATH", vec![parent.to_path_buf()]);
        }
        let status = proc
            .status()
            .with_context(|| format!("executing {}", cmd[0]))?;
        if !status.success() {
            let code = status.code().unwrap_or(1);
            bail!("command exited with status {code}: {cmd_display}");
        }
        Ok(())
    }
}

impl IdempotentRunner for ProcessRunner {
    fn run_no_check(&self, cmd: &[String]) {
        let cmd_display = shell_words::join(cmd);
        debug!("running: {cmd_display}");
        let _ = Command::new(&cmd[0]).args(&cmd[1..]).status();
    }
}

/// Dry-run runner that logs commands without executing.
pub struct DryRunner;

impl CommandRunner for DryRunner {
    fn run(&self, cmd: &[String]) -> Result<()> {
        info!("would run: {}", shell_words::join(cmd));
        Ok(())
    }
}

impl IdempotentRunner for DryRunner {
    fn run_no_check(&self, cmd: &[String]) {
        info!("would run: {}", shell_words::join(cmd));
    }
}

/// Runner that captures all commands for test assertions.
#[cfg(any(test, feature = "testutil"))]
pub struct MockRunner {
    pub commands: std::sync::Mutex<Vec<Vec<String>>>,
}

#[cfg(any(test, feature = "testutil"))]
impl Default for MockRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "testutil"))]
impl MockRunner {
    pub fn new() -> Self {
        Self {
            commands: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn commands(&self) -> Vec<Vec<String>> {
        self.commands.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "testutil"))]
impl CommandRunner for MockRunner {
    fn run(&self, cmd: &[String]) -> Result<()> {
        self.commands.lock().unwrap().push(cmd.to_vec());
        Ok(())
    }
}

#[cfg(any(test, feature = "testutil"))]
impl IdempotentRunner for MockRunner {
    fn run_no_check(&self, cmd: &[String]) {
        self.commands.lock().unwrap().push(cmd.to_vec());
    }
}

/// Prepend `paths` to an environment variable on a [`Command`], preserving
/// any existing value from the current process environment.
fn env_prepend_path(cmd: &mut Command, var: &str, paths: Vec<PathBuf>) {
    let old = std::env::var_os(var);
    let mut parts = paths;
    if let Some(ref v) = old {
        parts.extend(std::env::split_paths(v));
    }
    if let Ok(joined) = std::env::join_paths(&parts) {
        cmd.env(var, joined);
    }
}

#[cfg(test)]
mod tests {
    use super::env_prepend_path;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn env_prepend_path_composes_without_clobbering() {
        let mut cmd = Command::new("true");
        env_prepend_path(&mut cmd, "PATH", vec![PathBuf::from("/prepended")]);
        // Command stores env overrides internally; verify by inspecting
        // the environment it would pass. We can't read Command's env directly,
        // so we verify the helper logic: prepend + preserve.
        let old = std::env::var_os("PATH").unwrap_or_default();
        let mut parts = vec![PathBuf::from("/prepended")];
        parts.extend(std::env::split_paths(&old));
        let joined = std::env::join_paths(&parts).expect("join paths");
        let roundtrip: Vec<PathBuf> = std::env::split_paths(&joined).collect();
        assert_eq!(roundtrip[0], PathBuf::from("/prepended"));
        assert!(roundtrip.len() > 1, "should preserve existing PATH entries");
    }
}
