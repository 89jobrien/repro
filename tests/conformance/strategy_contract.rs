//! Contract tests for RuntimeStrategy port.

use repro::builder::{BuildConfig, DockerStrategy, PodmanStrategy, RuntimeStrategy};
use std::path::PathBuf;

fn base_config(runtime: &str) -> BuildConfig {
    BuildConfig {
        context: PathBuf::from("/tmp/ctx"),
        runtime: runtime.into(),
        runtime_path: PathBuf::from(format!("/usr/bin/{runtime}")),
        rootless: false,
        buildkit_image: "moby/buildkit:latest".into(),
        source_date_epoch: 1700000000,
        use_cache: true,
        file: None,
        output: PathBuf::from("/tmp/image.tar"),
        tag: Some("myapp:v1".into()),
        build_args: vec!["CI=true".into()],
        annotations: vec![],
        platform: None,
        buildkit_args: vec![],
        buildx_args: vec![],
    }
}

// --- C3.1: build_commands returns at least one CommandSpec ---

#[test]
fn c3_1_docker_strategy_returns_commands() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    assert!(!cmds.is_empty());
}

#[test]
fn c3_1_podman_strategy_returns_commands() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    assert!(!cmds.is_empty());
}

// --- C3.2: every CommandSpec.args is non-empty ---

#[test]
fn c3_2_docker_strategy_args_non_empty() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    for (i, spec) in cmds.iter().enumerate() {
        assert!(!spec.args.is_empty(), "docker command {i} has empty args");
    }
}

#[test]
fn c3_2_podman_strategy_args_non_empty() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    for (i, spec) in cmds.iter().enumerate() {
        assert!(!spec.args.is_empty(), "podman command {i} has empty args");
    }
}

// --- C3.3: SOURCE_DATE_EPOCH appears in commands ---

#[test]
fn c3_3_docker_strategy_includes_sde() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_sde = all_args
        .iter()
        .any(|a| a.contains("SOURCE_DATE_EPOCH=1700000000"));
    assert!(has_sde, "Docker commands must include SOURCE_DATE_EPOCH");
}

#[test]
fn c3_3_podman_strategy_includes_sde() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_sde = all_args
        .iter()
        .any(|a| a.contains("SOURCE_DATE_EPOCH=1700000000"));
    assert!(has_sde, "Podman commands must include SOURCE_DATE_EPOCH");
}

// --- C3.4: rewrite-timestamp=true in output option ---

#[test]
fn c3_4_docker_strategy_includes_rewrite_timestamp() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_rts = all_args
        .iter()
        .any(|a| a.contains("rewrite-timestamp=true"));
    assert!(
        has_rts,
        "Docker commands must include rewrite-timestamp=true"
    );
}

#[test]
fn c3_4_podman_strategy_includes_rewrite_timestamp() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_rts = all_args
        .iter()
        .any(|a| a.contains("rewrite-timestamp=true"));
    assert!(
        has_rts,
        "Podman commands must include rewrite-timestamp=true"
    );
}

// --- C3.5: tag appears when set ---

#[test]
fn c3_5_docker_strategy_includes_tag() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_tag = all_args.iter().any(|a| a.contains("myapp:v1"));
    assert!(has_tag, "Docker commands must include the tag");
}

#[test]
fn c3_5_podman_strategy_includes_tag() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_tag = all_args.iter().any(|a| a.contains("myapp:v1"));
    assert!(has_tag, "Podman commands must include the tag");
}

// --- C3.6: build_args appear when set ---

#[test]
fn c3_6_docker_strategy_includes_build_args() {
    let cmds = DockerStrategy.build_commands(&base_config("docker"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_ci = all_args.iter().any(|a| a.contains("CI=true"));
    assert!(has_ci, "Docker commands must include build args");
}

#[test]
fn c3_6_podman_strategy_includes_build_args() {
    let cmds = PodmanStrategy.build_commands(&base_config("podman"));
    let all_args: Vec<&String> = cmds.iter().flat_map(|c| &c.args).collect();
    let has_ci = all_args.iter().any(|a| a.contains("CI=true"));
    assert!(has_ci, "Podman commands must include build args");
}
