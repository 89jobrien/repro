//! Runtime-specific command construction strategies.

use super::config::BuildConfig;
use sha2::{Digest, Sha256};
use std::path::Path;

/// A command to execute, with metadata about whether failure is tolerable.
pub struct CommandSpec {
    pub args: Vec<String>,
    pub allow_failure: bool,
}

/// Runtime-specific command construction strategy.
pub trait RuntimeStrategy {
    fn build_commands(&self, config: &BuildConfig) -> Vec<CommandSpec>;
}

/// Docker Buildx command construction.
pub struct DockerStrategy;

impl DockerStrategy {
    /// Derive a deterministic builder name from the BuildKit image digest.
    fn builder_name(buildkit_image: &str) -> String {
        let id = hex::encode(Sha256::digest(buildkit_image.as_bytes()));
        format!("repro-build-{id}")
    }

    /// Construct the `docker buildx create` command.
    fn create_cmd(c: &BuildConfig, builder_name: &str) -> Vec<String> {
        vec![
            c.runtime_path.display().to_string(),
            "buildx".into(),
            "create".into(),
            "--name".into(),
            builder_name.into(),
            "--driver-opt".into(),
            format!("image={}", c.buildkit_image),
        ]
    }

    /// Construct the `docker buildx build` command.
    fn build_cmd(c: &BuildConfig, builder_name: String) -> Vec<String> {
        let mut cmd = vec![
            c.runtime_path.display().to_string(),
            "buildx".into(),
            "--builder".into(),
            builder_name,
            "build".into(),
            "--build-arg".into(),
            format!("SOURCE_DATE_EPOCH={}", c.source_date_epoch),
        ];

        for arg in &c.build_args {
            cmd.extend(["--build-arg".into(), arg.clone()]);
        }
        for arg in &c.annotations {
            cmd.extend(["--annotation".into(), arg.clone()]);
        }

        cmd.extend([
            "--provenance".into(),
            "false".into(),
            "--output".into(),
            format!(
                "type=docker,dest={},rewrite-timestamp=true",
                c.output.display()
            ),
        ]);

        if !c.use_cache {
            cmd.extend(["--no-cache".into(), "--pull".into()]);
        }
        if let Some(ref tag) = c.tag {
            cmd.extend(["-t".into(), tag.clone()]);
        }
        if let Some(ref file) = c.file {
            cmd.extend(["-f".into(), file.display().to_string()]);
        }
        if let Some(ref platform) = c.platform {
            cmd.extend(["--platform".into(), platform.clone()]);
        }

        cmd.extend(c.buildx_args.clone());
        cmd.push(c.context.display().to_string());
        cmd
    }
}

impl RuntimeStrategy for DockerStrategy {
    fn build_commands(&self, c: &BuildConfig) -> Vec<CommandSpec> {
        let name = Self::builder_name(&c.buildkit_image);
        vec![
            CommandSpec {
                args: Self::create_cmd(c, &name),
                allow_failure: true,
            },
            CommandSpec {
                args: Self::build_cmd(c, name),
                allow_failure: false,
            },
        ]
    }
}

/// Podman + BuildKit command construction.
pub struct PodmanStrategy;

impl PodmanStrategy {
    /// Build the `podman run` container arguments (volumes, security, entrypoint).
    fn container_args(c: &BuildConfig) -> (Vec<String>, Vec<String>) {
        let mut cmd = vec![
            c.runtime_path.display().to_string(),
            "run".into(),
            "-it".into(),
            "--rm".into(),
        ];

        cmd.extend(["-v".into(), "buildkit_cache:/tmp/cache".into()]);
        cmd.extend([
            "-v".into(),
            format!(
                "{}:/tmp/image",
                c.output
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .display()
            ),
        ]);
        cmd.extend(["-v".into(), format!("{}:/tmp/work", c.context.display())]);
        cmd.extend(["--entrypoint".into(), "buildctl-daemonless.sh".into()]);

        if c.rootless {
            cmd.extend([
                "--userns".into(),
                "keep-id:uid=1000,gid=1000".into(),
                "--security-opt".into(),
                "seccomp=unconfined".into(),
                "--security-opt".into(),
                "apparmor=unconfined".into(),
                "-e".into(),
                "BUILDKITD_FLAGS=--oci-worker-no-process-sandbox".into(),
            ]);
        } else {
            cmd.push("--privileged".into());
        }

        let mut dockerfile_args = Vec::new();
        if let Some(ref file) = c.file {
            cmd.extend(["-v".into(), format!("{}:/tmp/Dockerfile", file.display())]);
            dockerfile_args.extend(["--local".into(), "dockerfile=/tmp".into()]);
        } else {
            dockerfile_args.extend(["--local".into(), "dockerfile=/tmp/work".into()]);
        }

        cmd.push(c.buildkit_image.clone());
        (cmd, dockerfile_args)
    }

    /// Build the `buildctl build` arguments (frontend, build-args, output, cache).
    fn buildctl_args(c: &BuildConfig, dockerfile_args: Vec<String>) -> Vec<String> {
        let mut cmd = vec![
            "build".into(),
            "--frontend".into(),
            "dockerfile.v0".into(),
            "--local".into(),
            "context=/tmp/work".into(),
            "--opt".into(),
            format!("build-arg:SOURCE_DATE_EPOCH={}", c.source_date_epoch),
        ];

        for arg in &c.build_args {
            cmd.extend(["--opt".into(), format!("build-arg:{arg}")]);
        }

        let tag_suffix = c
            .tag
            .as_ref()
            .map(|t| format!(",name={t}"))
            .unwrap_or_default();
        let annotation_suffix: String = c
            .annotations
            .iter()
            .map(|a| format!(",annotation.{a}"))
            .collect();
        cmd.extend([
            "--output".into(),
            format!(
                "type=docker,dest=/tmp/image/{},rewrite-timestamp=true{tag_suffix}{annotation_suffix}",
                c.output.file_name().unwrap_or_default().to_string_lossy()
            ),
        ]);

        if c.use_cache {
            cmd.extend([
                "--export-cache".into(),
                "type=local,mode=max,dest=/tmp/cache".into(),
                "--import-cache".into(),
                "type=local,src=/tmp/cache".into(),
            ]);
        }

        cmd.extend(dockerfile_args);

        if let Some(ref platform) = c.platform {
            cmd.extend(["--opt".into(), format!("platform={platform}")]);
        }

        cmd.extend(c.buildkit_args.clone());
        cmd
    }
}

impl RuntimeStrategy for PodmanStrategy {
    fn build_commands(&self, c: &BuildConfig) -> Vec<CommandSpec> {
        let (mut cmd, dockerfile_args) = Self::container_args(c);
        cmd.extend(Self::buildctl_args(c, dockerfile_args));

        vec![CommandSpec {
            args: cmd,
            allow_failure: false,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config(runtime: &str) -> BuildConfig {
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
            annotations: vec!["org.test=true".into()],
            platform: None,
            buildkit_args: vec![],
            buildx_args: vec![],
        }
    }

    #[test]
    fn docker_strategy_produces_two_commands() {
        let cmds = DockerStrategy.build_commands(&test_config("docker"));
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].allow_failure);
        assert!(!cmds[1].allow_failure);
        assert!(cmds[0].args.contains(&"create".into()));
        assert!(cmds[1].args.contains(&"build".into()));
        assert!(
            cmds[1]
                .args
                .contains(&"SOURCE_DATE_EPOCH=1700000000".into())
        );
        assert!(cmds[1].args.contains(&"CI=true".into()));
        assert!(cmds[1].args.iter().any(|a| a.contains("myapp:v1")));
    }

    #[test]
    fn podman_strategy_produces_one_command() {
        let config = test_config("podman");
        let cmds = PodmanStrategy.build_commands(&config);
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].allow_failure);
        assert!(cmds[0].args.contains(&"run".into()));
        assert!(
            cmds[0]
                .args
                .iter()
                .any(|a| a.contains("build-arg:SOURCE_DATE_EPOCH=1700000000"))
        );
        assert!(cmds[0].args.iter().any(|a| a.contains("name=myapp:v1")));
    }
}
