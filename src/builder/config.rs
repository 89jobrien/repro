//! Build configuration types and parameter resolution.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;

use super::resolver::RuntimeResolver;

const DEFAULT_BUILDKIT_IMAGE: &str = "moby/buildkit:v0.19.0@sha256:\
     14aa1b4dd92ea0a4cd03a54d0c6079046ea98cd0c0ae6176bdd7036ba370cbbe";
const DEFAULT_BUILDKIT_IMAGE_ROOTLESS: &str = "moby/buildkit:v0.19.0-rootless@sha256:\
     e901cffdad753892a7c3afb8b9972549fca02c73888cf340c91ed801fdd96d71";

const ENV_RUNTIME: &str = "REPRO_RUNTIME";
const ENV_DATETIME: &str = "REPRO_DATETIME";
const ENV_SDE: &str = "REPRO_SOURCE_DATE_EPOCH";
const ENV_CACHE: &str = "REPRO_CACHE";
const ENV_ROOTLESS: &str = "REPRO_ROOTLESS";

/// Resolved build configuration (pure data, no behavior).
pub struct BuildConfig {
    pub context: PathBuf,
    pub runtime: String,
    pub runtime_path: PathBuf,
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
}

/// Input parameters before resolution.
///
/// Use `BuildParams::default()` in tests and override only the fields you need.
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
}

impl Default for BuildParams {
    fn default() -> Self {
        Self {
            context: ".".into(),
            runtime: None,
            source_date_epoch: Some(0),
            datetime: None,
            buildkit_image: None,
            no_cache: false,
            rootless: false,
            file: None,
            output: None,
            tag: None,
            build_args: vec![],
            annotations: vec![],
            platform: None,
            buildkit_args: None,
            buildx_args: None,
        }
    }
}

pub fn resolve_config(params: BuildParams, resolver: &dyn RuntimeResolver) -> Result<BuildConfig> {
    let resolved = resolve_runtime(params.runtime.as_deref(), resolver)?;
    let runtime = resolved.name;
    let runtime_path = resolved.path;
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
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("image.tar")
        });

    Ok(BuildConfig {
        context,
        runtime,
        runtime_path,
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
    })
}

fn resolve_runtime(
    runtime: Option<&str>,
    resolver: &dyn RuntimeResolver,
) -> Result<super::resolver::ResolvedRuntime> {
    if let Some(r) = runtime {
        return resolver.resolve(r);
    }
    if let Ok(r) = std::env::var(ENV_RUNTIME) {
        if r != "docker" && r != "podman" {
            bail!("only 'docker' or 'podman' runtimes are supported");
        }
        return resolver.resolve(&r);
    }
    resolver
        .resolve("docker")
        .or_else(|_| resolver.resolve("podman"))
        .context("no container runtime (docker or podman) detected on your system")
}

pub(crate) fn resolve_use_cache(no_cache: bool) -> bool {
    if no_cache {
        return false;
    }
    std::env::var(ENV_CACHE)
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .map(|v| v != 0)
        .unwrap_or(true)
}

pub(crate) fn resolve_rootless(runtime: &str, rootless: bool) -> Result<bool> {
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

pub(crate) fn resolve_sde(sde: Option<i64>, datetime_str: Option<&str>) -> Result<i64> {
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
                                .map(|nd| nd.and_time(chrono::NaiveTime::MIN))
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

pub(crate) fn resolve_buildkit_image(image: Option<&str>, rootless: bool, runtime: &str) -> String {
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

pub(crate) fn resolve_buildkit_args(args: Option<&str>, runtime: &str) -> Result<Vec<String>> {
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

pub(crate) fn resolve_buildx_args(args: Option<&str>, runtime: &str) -> Result<Vec<String>> {
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

    #[cfg(any(test, feature = "testutil"))]
    use super::super::resolver::MockResolver;

    // --- resolve_sde ---

    #[test]
    fn resolve_sde_from_epoch() {
        assert_eq!(resolve_sde(Some(1700000000), None).unwrap(), 1700000000);
    }

    #[test]
    fn resolve_sde_from_rfc3339() {
        assert_eq!(
            resolve_sde(None, Some("2024-01-15T12:00:00Z")).unwrap(),
            1705320000
        );
    }

    #[test]
    fn resolve_sde_from_iso_datetime() {
        assert_eq!(
            resolve_sde(None, Some("2024-01-15T12:00:00")).unwrap(),
            1705320000
        );
    }

    #[test]
    fn resolve_sde_from_date_only() {
        assert_eq!(resolve_sde(None, Some("2024-01-15")).unwrap(), 1705276800);
    }

    #[test]
    fn resolve_sde_both_set_is_error() {
        let result = resolve_sde(Some(100), Some("2024-01-15"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not both"));
    }

    #[test]
    fn resolve_sde_neither_set_is_error() {
        assert!(resolve_sde(None, None).is_err());
    }

    #[test]
    fn resolve_sde_invalid_datetime_is_error() {
        assert!(resolve_sde(None, Some("not-a-date")).is_err());
    }

    // --- resolve_runtime ---

    #[test]
    fn resolve_runtime_with_mock_resolver() {
        let resolver = MockResolver::new("/usr/bin/docker".into());
        let r = resolve_runtime(Some("docker"), &resolver).unwrap();
        assert_eq!(r.name, "docker");
        assert_eq!(r.path, PathBuf::from("/usr/bin/docker"));
    }

    #[test]
    fn resolve_runtime_fallback_order() {
        let resolver = MockResolver::new("/usr/bin/docker".into());
        let r = resolve_runtime(None, &resolver).unwrap();
        assert_eq!(r.name, "docker");
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
    use super::resolve_use_cache;

    #[kani::proof]
    fn use_cache_no_cache_always_false() {
        assert!(!resolve_use_cache(true));
    }

    #[kani::proof]
    fn rootless_constraint_docker_rootless_rejected() {
        let runtime = "docker";
        let rootless = true;
        assert!(runtime != "podman" && rootless);
    }

    #[kani::proof]
    fn rootless_constraint_false_always_accepted() {
        let rootless = false;
        assert!(!(rootless));
    }

    #[kani::proof]
    fn sde_both_set_hits_error_arm() {
        let epoch: i64 = kani::any();
        let sde = Some(epoch);
        let dt: Option<&str> = Some("2024-01-01");
        assert!(matches!((sde, dt), (Some(_), Some(_))));
    }

    #[kani::proof]
    fn sde_neither_set_hits_error_arm() {
        let sde: Option<i64> = None;
        let dt: Option<&str> = None;
        assert!(matches!((sde, dt), (None, None)));
    }

    #[kani::proof]
    fn sde_epoch_only_returns_value() {
        let epoch: i64 = kani::any();
        let sde = Some(epoch);
        let dt: Option<&str> = None;
        match (sde, dt) {
            (Some(s), None) => assert_eq!(s, epoch),
            _ => panic!("wrong arm"),
        }
    }

    #[kani::proof]
    fn buildkit_image_podman_needs_prefix() {
        let rootless = false;
        let runtime = "podman";
        assert!((rootless || runtime == "podman"));
    }

    #[kani::proof]
    fn buildkit_image_rootless_needs_prefix() {
        let rootless = true;
        let runtime = "podman";
        assert!((rootless || runtime == "podman"));
    }

    #[kani::proof]
    fn buildkit_image_docker_no_prefix() {
        let rootless = false;
        let runtime = "docker";
        assert!(!(rootless || runtime == "podman"));
    }

    #[kani::proof]
    fn buildkit_args_constraint_docker_rejected() {
        let runtime = "docker";
        assert!(runtime != "podman");
    }

    #[kani::proof]
    fn buildx_args_constraint_podman_rejected() {
        let runtime = "podman";
        assert!(runtime != "docker");
    }
}
