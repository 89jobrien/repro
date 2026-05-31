use repro::builder::{BuildParams, Builder, MockResolver, MockRunner};
use repro::oci;
use repro::testutil::OciTarballFixture;
// ---------------------------------------------------------------------------
// Builder + MockRunner + MockResolver: no docker/podman needed
// ---------------------------------------------------------------------------

#[test]
fn mock_runner_captures_docker_commands() {
    let resolver = MockResolver::new("/usr/bin/docker".into());
    let runner = MockRunner::new();
    let params = BuildParams {
        runtime: Some("docker".into()),
        source_date_epoch: Some(1700000000),
        tag: Some("myapp:v1".into()),
        build_args: vec!["CI=true".into()],
        ..BuildParams::default()
    };

    let builder = Builder::with_deps(params, &resolver, runner).expect("should resolve");
    builder.build().expect("mock build should succeed");

    let cmds = builder.runner.commands();
    assert_eq!(cmds.len(), 2, "expected create + build commands");
    assert!(cmds[0].contains(&"create".into()));
    assert!(cmds[1].contains(&"build".into()));
    assert!(cmds[1].contains(&"SOURCE_DATE_EPOCH=1700000000".into()));
    assert!(cmds[1].contains(&"CI=true".into()));
    assert!(cmds[1].iter().any(|a| a.contains("myapp:v1")));
    assert!(cmds[1].iter().any(|a| a.contains("rewrite-timestamp=true")));
}

#[test]
fn mock_runner_captures_podman_commands() {
    let resolver = MockResolver::new("/usr/bin/podman".into());
    let runner = MockRunner::new();
    let params = BuildParams {
        runtime: Some("podman".into()),
        source_date_epoch: Some(1700000000),
        tag: Some("myapp:v1".into()),
        annotations: vec!["org.test=true".into()],
        ..BuildParams::default()
    };

    let builder = Builder::with_deps(params, &resolver, runner).expect("should resolve");
    builder.build().expect("mock build should succeed");

    let cmds = builder.runner.commands();
    assert_eq!(cmds.len(), 1, "podman emits one command");
    assert!(cmds[0].contains(&"run".into()));
    assert!(
        cmds[0]
            .iter()
            .any(|a| a.contains("build-arg:SOURCE_DATE_EPOCH=1700000000"))
    );
    assert!(cmds[0].iter().any(|a| a.contains("name=myapp:v1")));
    assert!(
        cmds[0]
            .iter()
            .any(|a| a.contains("annotation.org.test=true"))
    );
}

#[test]
fn mock_runner_docker_no_cache_adds_flags() {
    let resolver = MockResolver::new("/usr/bin/docker".into());
    let runner = MockRunner::new();
    let params = BuildParams {
        runtime: Some("docker".into()),
        source_date_epoch: Some(0),
        no_cache: true,
        ..BuildParams::default()
    };

    let builder = Builder::with_deps(params, &resolver, runner).expect("should resolve");
    builder.build().expect("mock build should succeed");

    let build_cmd = &builder.runner.commands()[1];
    assert!(build_cmd.contains(&"--no-cache".into()));
    assert!(build_cmd.contains(&"--pull".into()));
}

#[test]
fn build_params_default_has_sensible_values() {
    let params = BuildParams::default();
    assert_eq!(params.source_date_epoch, Some(0));
    assert_eq!(params.context, ".");
}

// ---------------------------------------------------------------------------
// Builder dry-run (ProcessRunner path, needs real docker)
// ---------------------------------------------------------------------------

#[test]
fn builder_dry_run_resolves() {
    if which::which("docker").is_err() {
        eprintln!("skipping: docker not found");
        return;
    }

    let params = BuildParams {
        runtime: Some("docker".into()),
        source_date_epoch: Some(1700000000),
        output: Some("/tmp/repro-test-integration.tar".into()),
        tag: Some("test:latest".into()),
        build_args: vec!["FOO=bar".into()],
        annotations: vec!["org.test=true".into()],
        ..BuildParams::default()
    };

    let builder = Builder::dry(params).expect("Builder::dry should resolve");
    assert_eq!(builder.config.runtime, "docker");
    assert_eq!(builder.config.source_date_epoch, 1700000000);
    builder.build().expect("dry build should succeed");
}

// ---------------------------------------------------------------------------
// OCI tarball fixture tests
// ---------------------------------------------------------------------------

#[test]
fn fixture_default_tarball_parses() {
    let (tmpfile, digests) = OciTarballFixture::default().build();
    let parsed = oci::parse_tarball(tmpfile.path()).expect("should parse");
    assert_eq!(parsed.len(), 2, "index + 1 manifest");
    assert_eq!(parsed[0].path, "index.json");
    assert_eq!(parsed[1].digest, digests[0]);
}

#[test]
fn fixture_dot_slash_prefix_parses() {
    let (tmpfile, digests) = OciTarballFixture::default().with_dot_slash_prefix().build();
    let parsed = oci::parse_tarball(tmpfile.path()).expect("should parse with ./ prefix");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[1].digest, digests[0]);
}

#[test]
fn fixture_multi_manifest_parses() {
    let manifest_a = r#"{"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"digest":"sha256:aaaa"},"layers":[]}"#;
    let manifest_b = r#"{"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"digest":"sha256:bbbb"},"layers":[]}"#;

    let (tmpfile, digests) = OciTarballFixture::default()
        .with_manifest(manifest_a)
        .add_manifest(manifest_b, "linux", "amd64")
        .build();

    let parsed = oci::parse_tarball(tmpfile.path()).expect("should parse multi-manifest");
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[1].digest, digests[0]);
    assert_eq!(parsed[2].digest, digests[1]);
    assert_eq!(parsed[2].platform.as_deref(), Some("linux/amd64"));
}

#[test]
fn fixture_verify_digest_match_and_mismatch() {
    let (tmpfile, digests) = OciTarballFixture::default().build();
    let parsed = oci::parse_tarball(tmpfile.path()).expect("should parse");

    oci::verify_digest(&parsed, &digests[0]).expect("should match with prefix");

    let hash = digests[0].strip_prefix("sha256:").unwrap();
    oci::verify_digest(&parsed, hash).expect("should match without prefix");

    assert!(oci::verify_digest(&parsed, "sha256:0000dead").is_err());
}

#[test]
fn fixture_gzip_tarball_parses() {
    let (tmpfile, digests) = OciTarballFixture::default().with_gzip().build();
    let parsed = oci::parse_tarball(tmpfile.path()).expect("should parse gzip tarball");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[1].digest, digests[0]);
}
