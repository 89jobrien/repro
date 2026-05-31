//! Contract tests for Builder integration (port composition).

use repro::builder::{BuildParams, Builder, MockResolver, MockRunner};
use std::path::PathBuf;

fn docker_params() -> BuildParams {
    BuildParams {
        context: "/tmp".into(),
        runtime: Some("docker".into()),
        source_date_epoch: Some(1700000000),
        ..Default::default()
    }
}

fn podman_params() -> BuildParams {
    BuildParams {
        context: "/tmp".into(),
        runtime: Some("podman".into()),
        source_date_epoch: Some(1700000000),
        ..Default::default()
    }
}

// --- C4.1: Builder invokes runner with strategy commands ---

#[test]
fn c4_1_builder_docker_invokes_runner() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/docker"));
    let runner = MockRunner::new();
    let builder = Builder::with_deps(docker_params(), &resolver, runner).unwrap();
    builder.build().unwrap();
    let cmds = builder.runner.commands();
    assert!(
        cmds.len() >= 2,
        "Docker builder should invoke at least 2 commands (create + build)"
    );
}

#[test]
fn c4_1_builder_podman_invokes_runner() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/podman"));
    let runner = MockRunner::new();
    let builder = Builder::with_deps(podman_params(), &resolver, runner).unwrap();
    builder.build().unwrap();
    let cmds = builder.runner.commands();
    assert!(
        !cmds.is_empty(),
        "Podman builder should invoke at least 1 command"
    );
}

// --- C4.2: Docker commands are ordered (create before build) ---

#[test]
fn c4_2_docker_create_before_build() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/docker"));
    let runner = MockRunner::new();
    let builder = Builder::with_deps(docker_params(), &resolver, runner).unwrap();
    builder.build().unwrap();
    let cmds = builder.runner.commands();
    assert!(
        cmds[0].contains(&"create".to_string()),
        "first command should be create"
    );
    assert!(
        cmds[1].contains(&"build".to_string()),
        "second command should be build"
    );
}

// --- C4.3: Runner error propagates from build() ---

#[test]
fn c4_3_runner_error_propagates() {
    use anyhow::{Result, bail};
    use repro::builder::{CommandRunner, IdempotentRunner};

    struct FailRunner;
    impl CommandRunner for FailRunner {
        fn run(&self, _cmd: &[String]) -> Result<()> {
            bail!("simulated failure")
        }
    }
    impl IdempotentRunner for FailRunner {
        fn run_no_check(&self, _cmd: &[String]) {}
    }

    let resolver = MockResolver::new(PathBuf::from("/usr/bin/podman"));
    let builder = Builder::with_deps(podman_params(), &resolver, FailRunner).unwrap();
    let result = builder.build();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("simulated failure")
    );
}

// --- C4.4: Invalid runtime returns error ---

#[test]
fn c4_4_invalid_runtime_errors() {
    let resolver = MockResolver::new(PathBuf::from("/usr/bin/invalid"));
    let runner = MockRunner::new();
    let params = BuildParams {
        context: "/tmp".into(),
        runtime: Some("invalid".into()),
        source_date_epoch: Some(0),
        ..Default::default()
    };
    let builder = Builder::with_deps(params, &resolver, runner).unwrap();
    let result = builder.build();
    assert!(result.is_err());
}
