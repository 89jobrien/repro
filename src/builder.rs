//! Reproducible container image builder.
//!
//! Drives Docker Buildx or Podman+BuildKit to produce deterministic OCI
//! image tarballs using `SOURCE_DATE_EPOCH` and `rewrite-timestamp=true`.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

const DEFAULT_BUILDKIT_IMAGE: &str = "moby/buildkit:v0.19.0@sha256:\
     14aa1b4dd92ea0a4cd03a54d0c6079046ea98cd0c0ae6176bdd7036ba370cbbe";
const DEFAULT_BUILDKIT_IMAGE_ROOTLESS: &str = "moby/buildkit:v0.19.0-rootless@sha256:\
     e901cffdad753892a7c3afb8b9972549fca02c73888cf340c91ed801fdd96d71";

const ENV_RUNTIME: &str = "REPRO_RUNTIME";
const ENV_DATETIME: &str = "REPRO_DATETIME";
const ENV_SDE: &str = "REPRO_SOURCE_DATE_EPOCH";
const ENV_CACHE: &str = "REPRO_CACHE";
const ENV_ROOTLESS: &str = "REPRO_ROOTLESS";

/// Resolved build configuration.
pub struct Builder {
    pub context: PathBuf,
    pub runtime: String,
    pub rootless: bool,
    pub buildkit_image: String,
    pub source_date_epoch: i64,
    pub use_cache: bool,
    pub file: Option<PathBuf>,
    pub output: PathBuf,
    pub tag: Option<String>,
    pub build_args: Vec<String>,
    pub annotations: Vec<String>,
    pub platform: Option<String>,
    pub buildkit_args: Vec<String>,
    pub buildx_args: Vec<String>,
    pub dry: bool,
}

/// Input parameters before resolution.
pub struct BuildParams {
    pub context: String,
    pub runtime: Option<String>,
    pub source_date_epoch: Option<i64>,
    pub datetime: Option<String>,
    pub buildkit_image: Option<String>,
    pub no_cache: bool,
    pub rootless: bool,
    pub file: Option<String>,
    pub output: Option<String>,
    pub tag: Option<String>,
    pub build_args: Vec<String>,
    pub annotations: Vec<String>,
    pub platform: Option<String>,
    pub buildkit_args: Option<String>,
    pub buildx_args: Option<String>,
    pub dry: bool,
}

impl Builder {
    /// Create a builder from unresolved parameters.
    pub fn new(params: BuildParams) -> Result<Self> {
        let runtime = resolve_runtime(params.runtime.as_deref())?;
        let rootless = resolve_rootless(&runtime, params.rootless)?;
        let buildkit_image =
            resolve_buildkit_image(params.buildkit_image.as_deref(), rootless, &runtime);
        let source_date_epoch = resolve_sde(params.source_date_epoch, params.datetime.as_deref())?;
        let use_cache = resolve_use_cache(params.no_cache);
        let buildkit_args = resolve_buildkit_args(params.buildkit_args.as_deref(), &runtime)?;
        let buildx_args = resolve_buildx_args(params.buildx_args.as_deref(), &runtime)?;

        let context = std::fs::canonicalize(&params.context)
            .with_context(|| format!("resolving context path: {}", params.context))?;

        let file = params
            .file
            .as_ref()
            .map(|f| std::fs::canonicalize(f).with_context(|| format!("resolving file: {f}")))
            .transpose()?;

        let output = params
            .output
            .as_ref()
            .map(|o| {
                PathBuf::from(o)
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from(o))
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap().join("image.tar"));

        Ok(Self {
            context,
            runtime,
            rootless,
            buildkit_image,
            source_date_epoch,
            use_cache,
            file,
            output,
            tag: params.tag,
            build_args: params.build_args,
            annotations: params.annotations,
            platform: params.platform,
            buildkit_args,
            buildx_args,
            dry: params.dry,
        })
    }

    /// Execute the reproducible build.
    pub fn build(&self) -> Result<()> {
        self.log_context();
        match self.runtime.as_str() {
            "docker" => self.docker_build(),
            "podman" => self.podman_build(),
            other => bail!("unsupported runtime: {other}"),
        }
    }

    fn log_context(&self) {
        info!(
            runtime = %self.runtime,
            buildkit_image = %self.buildkit_image,
            rootless = self.rootless,
            use_cache = self.use_cache,
            context = %self.context.display(),
            dockerfile = %self.file.as_deref().map(|p| p.display().to_string())
                .unwrap_or_else(|| "(not provided)".into()),
            output = %self.output.display(),
            sde = self.source_date_epoch,
            tag = %self.tag.as_deref().unwrap_or("(not provided)"),
            build_args = %if self.build_args.is_empty() {
                "(not provided)".into()
            } else {
                self.build_args.join(",")
            },
            platform = %self.platform.as_deref().unwrap_or("(default)"),
            "build: configuration resolved"
        );
    }

    fn podman_build(&self) -> Result<()> {
        let mut cmd = vec!["podman".into(), "run".into(), "-it".into(), "--rm".into()];

        // Volume mounts
        cmd.extend(["-v".into(), "buildkit_cache:/tmp/cache".into()]);
        cmd.extend([
            "-v".into(),
            format!(
                "{}:/tmp/image",
                self.output
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .display()
            ),
        ]);
        cmd.extend(["-v".into(), format!("{}:/tmp/work", self.context.display())]);
        cmd.extend(["--entrypoint".into(), "buildctl-daemonless.sh".into()]);

        // Rootless vs rootful
        if self.rootless {
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

        // Dockerfile mounting
        let mut buildkit_dockerfile_args = Vec::new();
        if let Some(ref file) = self.file {
            cmd.extend(["-v".into(), format!("{}:/tmp/Dockerfile", file.display())]);
            buildkit_dockerfile_args.extend(["--local".into(), "dockerfile=/tmp".into()]);
        } else {
            buildkit_dockerfile_args.extend(["--local".into(), "dockerfile=/tmp/work".into()]);
        }

        // BuildKit image
        cmd.push(self.buildkit_image.clone());

        // BuildKit command
        cmd.extend([
            "build".into(),
            "--frontend".into(),
            "dockerfile.v0".into(),
            "--local".into(),
            "context=/tmp/work".into(),
            "--opt".into(),
            format!("build-arg:SOURCE_DATE_EPOCH={}", self.source_date_epoch),
        ]);

        // Build args
        for arg in &self.build_args {
            cmd.extend(["--opt".into(), format!("build-arg:{arg}")]);
        }

        // Output with tag and annotations
        let tag_suffix = self
            .tag
            .as_ref()
            .map(|t| format!(",name={t}"))
            .unwrap_or_default();
        let annotation_suffix: String = self
            .annotations
            .iter()
            .map(|a| format!(",annotation.{a}"))
            .collect();
        cmd.extend([
            "--output".into(),
            format!(
                "type=docker,dest=/tmp/image/{},rewrite-timestamp=true{tag_suffix}{annotation_suffix}",
                self.output.file_name().unwrap_or_default().to_string_lossy()
            ),
        ]);

        // Cache
        if self.use_cache {
            cmd.extend([
                "--export-cache".into(),
                "type=local,mode=max,dest=/tmp/cache".into(),
                "--import-cache".into(),
                "type=local,src=/tmp/cache".into(),
            ]);
        }

        cmd.extend(buildkit_dockerfile_args);

        // Platform
        if let Some(ref platform) = self.platform {
            cmd.extend(["--opt".into(), format!("platform={platform}")]);
        }

        // Extra BuildKit args
        cmd.extend(self.buildkit_args.clone());

        run_cmd(&cmd, self.dry)
    }

    fn docker_build(&self) -> Result<()> {
        let builder_id = hex::encode(Sha256::digest(self.buildkit_image.as_bytes()));
        let builder_name = format!("repro-build-{builder_id}");

        // Create builder (ignore failure if it already exists)
        let create_cmd = vec![
            "docker".into(),
            "buildx".into(),
            "create".into(),
            "--name".into(),
            builder_name.clone(),
            "--driver-opt".into(),
            format!("image={}", self.buildkit_image),
        ];
        run_cmd_no_check(&create_cmd, self.dry);

        let mut cmd = vec![
            "docker".into(),
            "buildx".into(),
            "--builder".into(),
            builder_name,
            "build".into(),
            "--build-arg".into(),
            format!("SOURCE_DATE_EPOCH={}", self.source_date_epoch),
        ];

        // Build args
        for arg in &self.build_args {
            cmd.extend(["--build-arg".into(), arg.clone()]);
        }

        // Annotations
        for arg in &self.annotations {
            cmd.extend(["--annotation".into(), arg.clone()]);
        }

        cmd.extend([
            "--provenance".into(),
            "false".into(),
            "--output".into(),
            format!(
                "type=docker,dest={},rewrite-timestamp=true",
                self.output.display()
            ),
        ]);

        // Cache
        if !self.use_cache {
            cmd.extend(["--no-cache".into(), "--pull".into()]);
        }

        // Tag
        if let Some(ref tag) = self.tag {
            cmd.extend(["-t".into(), tag.clone()]);
        }

        // Dockerfile
        if let Some(ref file) = self.file {
            cmd.extend(["-f".into(), file.display().to_string()]);
        }

        // Platform
        if let Some(ref platform) = self.platform {
            cmd.extend(["--platform".into(), platform.clone()]);
        }

        // Extra buildx args
        cmd.extend(self.buildx_args.clone());

        // Context (last)
        cmd.push(self.context.display().to_string());

        run_cmd(&cmd, self.dry)
    }
}

fn run_cmd(cmd: &[String], dry: bool) -> Result<()> {
    let cmd_display = shell_words::join(cmd);
    if dry {
        info!("would run: {cmd_display}");
        return Ok(());
    }
    debug!("running: {cmd_display}");
    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .with_context(|| format!("executing {}", cmd[0]))?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("command exited with status {code}: {cmd_display}");
    }
    Ok(())
}

fn run_cmd_no_check(cmd: &[String], dry: bool) {
    let cmd_display = shell_words::join(cmd);
    if dry {
        info!("would run: {cmd_display}");
        return;
    }
    debug!("running: {cmd_display}");
    let _ = Command::new(&cmd[0]).args(&cmd[1..]).status();
}

fn detect_container_runtime() -> Option<String> {
    if which::which("docker").is_ok() {
        Some("docker".into())
    } else if which::which("podman").is_ok() {
        Some("podman".into())
    } else {
        None
    }
}

fn resolve_runtime(runtime: Option<&str>) -> Result<String> {
    if let Some(r) = runtime {
        return Ok(r.to_string());
    }
    if let Ok(r) = std::env::var(ENV_RUNTIME) {
        if r != "docker" && r != "podman" {
            bail!("only 'docker' or 'podman' runtimes are supported");
        }
        return Ok(r);
    }
    detect_container_runtime()
        .context("no container runtime (docker or podman) detected on your system")
}

fn resolve_use_cache(no_cache: bool) -> bool {
    if no_cache {
        return false;
    }
    std::env::var(ENV_CACHE)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v != 0)
        .unwrap_or(true)
}

fn resolve_rootless(runtime: &str, rootless: bool) -> Result<bool> {
    let env_rootless = std::env::var(ENV_ROOTLESS)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v != 0)
        .unwrap_or(false);
    let rootless = rootless || env_rootless;
    if runtime != "podman" && rootless {
        bail!("rootless mode is only supported with Podman runtime");
    }
    Ok(rootless)
}

fn resolve_sde(sde: Option<i64>, datetime_str: Option<&str>) -> Result<i64> {
    let env_sde = std::env::var(ENV_SDE).ok().and_then(|v| v.parse().ok());
    let env_dt = std::env::var(ENV_DATETIME).ok();

    let sde = sde.or(env_sde);
    let dt: Option<&str> = datetime_str.or(env_dt.as_deref());

    match (sde, dt) {
        (Some(s), None) => Ok(s),
        (None, Some(d)) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(d)
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(d, "%Y-%m-%dT%H:%M:%S")
                        .or_else(|_| {
                            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                                .map(|nd| nd.and_hms_opt(0, 0, 0).unwrap())
                        })
                        .map(|ndt| ndt.and_utc().fixed_offset())
                })
                .with_context(|| format!("parsing datetime: {d}"))?;
            Ok(parsed.timestamp())
        }
        (Some(_), Some(_)) => {
            bail!("pass either --source-date-epoch or --datetime, not both")
        }
        (None, None) => {
            bail!("you must pass either --source-date-epoch or --datetime")
        }
    }
}

fn resolve_buildkit_image(image: Option<&str>, rootless: bool, runtime: &str) -> String {
    let mut img = match image {
        Some(i) => i.to_string(),
        None if rootless => DEFAULT_BUILDKIT_IMAGE_ROOTLESS.to_string(),
        None => DEFAULT_BUILDKIT_IMAGE.to_string(),
    };
    if (rootless || runtime == "podman") && !img.starts_with("docker.io/") {
        img = format!("docker.io/{img}");
    }
    img
}

fn resolve_buildkit_args(args: Option<&str>, runtime: &str) -> Result<Vec<String>> {
    match args {
        None | Some("") => Ok(vec![]),
        Some(a) => {
            if runtime != "podman" {
                bail!("cannot specify BuildKit arguments with the Docker runtime");
            }
            Ok(shell_words::split(a).context("parsing buildkit args")?)
        }
    }
}

fn resolve_buildx_args(args: Option<&str>, runtime: &str) -> Result<Vec<String>> {
    match args {
        None | Some("") => Ok(vec![]),
        Some(a) => {
            if runtime != "docker" {
                bail!("cannot specify Docker Buildx arguments with the Podman runtime");
            }
            Ok(shell_words::split(a).context("parsing buildx args")?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- resolve_sde ---

    #[test]
    fn resolve_sde_from_epoch() {
        let result = resolve_sde(Some(1700000000), None).expect("should resolve");
        assert_eq!(result, 1700000000);
    }

    #[test]
    fn resolve_sde_from_rfc3339() {
        let result = resolve_sde(None, Some("2024-01-15T12:00:00Z")).expect("should parse RFC3339");
        assert_eq!(result, 1705320000);
    }

    #[test]
    fn resolve_sde_from_iso_datetime() {
        let result =
            resolve_sde(None, Some("2024-01-15T12:00:00")).expect("should parse ISO datetime");
        assert_eq!(result, 1705320000);
    }

    #[test]
    fn resolve_sde_from_date_only() {
        let result = resolve_sde(None, Some("2024-01-15")).expect("should parse date");
        assert_eq!(result, 1705276800);
    }

    #[test]
    fn resolve_sde_both_set_is_error() {
        let result = resolve_sde(Some(100), Some("2024-01-15"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not both"));
    }

    #[test]
    fn resolve_sde_neither_set_is_error() {
        let result = resolve_sde(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_sde_invalid_datetime_is_error() {
        let result = resolve_sde(None, Some("not-a-date"));
        assert!(result.is_err());
    }

    // --- resolve_runtime ---

    #[test]
    fn resolve_runtime_explicit() {
        assert_eq!(resolve_runtime(Some("docker")).unwrap(), "docker");
        assert_eq!(resolve_runtime(Some("podman")).unwrap(), "podman");
    }

    // --- resolve_use_cache ---

    #[test]
    fn resolve_use_cache_defaults_true() {
        assert!(resolve_use_cache(false));
    }

    #[test]
    fn resolve_use_cache_no_cache_flag() {
        assert!(!resolve_use_cache(true));
    }

    // --- resolve_rootless ---

    #[test]
    fn resolve_rootless_podman_ok() {
        assert!(resolve_rootless("podman", true).unwrap());
    }

    #[test]
    fn resolve_rootless_docker_is_error() {
        assert!(resolve_rootless("docker", true).is_err());
    }

    #[test]
    fn resolve_rootless_false_always_ok() {
        assert!(!resolve_rootless("docker", false).unwrap());
        assert!(!resolve_rootless("podman", false).unwrap());
    }

    // --- resolve_buildkit_image ---

    #[test]
    fn resolve_buildkit_image_default() {
        let img = resolve_buildkit_image(None, false, "docker");
        assert!(img.contains("moby/buildkit"));
        assert!(!img.starts_with("docker.io/"));
    }

    #[test]
    fn resolve_buildkit_image_rootless_default() {
        let img = resolve_buildkit_image(None, true, "podman");
        assert!(img.contains("rootless"));
        assert!(img.starts_with("docker.io/"));
    }

    #[test]
    fn resolve_buildkit_image_podman_adds_prefix() {
        let img = resolve_buildkit_image(Some("moby/buildkit:latest"), false, "podman");
        assert!(img.starts_with("docker.io/"));
    }

    #[test]
    fn resolve_buildkit_image_custom_already_prefixed() {
        let img = resolve_buildkit_image(Some("docker.io/moby/buildkit:latest"), false, "podman");
        assert_eq!(img, "docker.io/moby/buildkit:latest");
    }

    // --- resolve_buildkit_args / resolve_buildx_args ---

    #[test]
    fn resolve_buildkit_args_empty() {
        assert!(resolve_buildkit_args(None, "podman").unwrap().is_empty());
        assert!(
            resolve_buildkit_args(Some(""), "podman")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn resolve_buildkit_args_docker_is_error() {
        assert!(resolve_buildkit_args(Some("--foo"), "docker").is_err());
    }

    #[test]
    fn resolve_buildkit_args_podman_splits() {
        let args = resolve_buildkit_args(Some("--foo --bar baz"), "podman").unwrap();
        assert_eq!(args, vec!["--foo", "--bar", "baz"]);
    }

    #[test]
    fn resolve_buildx_args_empty() {
        assert!(resolve_buildx_args(None, "docker").unwrap().is_empty());
    }

    #[test]
    fn resolve_buildx_args_podman_is_error() {
        assert!(resolve_buildx_args(Some("--foo"), "podman").is_err());
    }

    #[test]
    fn resolve_buildx_args_docker_splits() {
        let args = resolve_buildx_args(Some("--foo bar"), "docker").unwrap();
        assert_eq!(args, vec!["--foo", "bar"]);
    }

    // --- property tests ---

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn sde_roundtrip_from_epoch(epoch in 0i64..=4_102_444_800i64) {
                let result = resolve_sde(Some(epoch), None).unwrap();
                prop_assert_eq!(result, epoch);
            }

            #[test]
            fn rootless_false_never_errors(runtime in "(docker|podman)") {
                prop_assert!(resolve_rootless(&runtime, false).is_ok());
            }

            #[test]
            fn use_cache_no_cache_always_false(no_cache in proptest::bool::ANY) {
                if no_cache {
                    prop_assert!(!resolve_use_cache(true));
                }
            }
        }
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn use_cache_no_cache_always_false() {
        assert!(!resolve_use_cache(true));
    }

    #[kani::proof]
    fn rootless_docker_always_errors() {
        assert!(resolve_rootless("docker", true).is_err());
    }

    #[kani::proof]
    fn rootless_false_never_errors() {
        assert!(resolve_rootless("docker", false).is_ok());
        assert!(resolve_rootless("podman", false).is_ok());
    }

    #[kani::proof]
    fn rootless_podman_ok() {
        assert!(resolve_rootless("podman", true).is_ok());
    }

    #[kani::proof]
    fn sde_both_set_always_errors() {
        let epoch: i64 = kani::any();
        assert!(resolve_sde(Some(epoch), Some("2024-01-01")).is_err());
    }

    #[kani::proof]
    fn sde_neither_set_always_errors() {
        assert!(resolve_sde(None, None).is_err());
    }

    #[kani::proof]
    fn sde_epoch_identity() {
        let epoch: i64 = kani::any();
        let result = resolve_sde(Some(epoch), None).unwrap();
        assert_eq!(result, epoch);
    }

    #[kani::proof]
    fn buildkit_image_podman_has_prefix() {
        let img = resolve_buildkit_image(None, false, "podman");
        assert!(img.starts_with("docker.io/"));
    }

    #[kani::proof]
    fn buildkit_image_rootless_has_prefix() {
        let img = resolve_buildkit_image(None, true, "podman");
        assert!(img.starts_with("docker.io/"));
    }

    #[kani::proof]
    fn buildkit_args_docker_errors() {
        assert!(resolve_buildkit_args(Some("--foo"), "docker").is_err());
    }

    #[kani::proof]
    fn buildx_args_podman_errors() {
        assert!(resolve_buildx_args(Some("--foo"), "podman").is_err());
    }
}
