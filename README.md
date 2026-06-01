# repro

[![Crates.io](https://img.shields.io/crates/v/repro)](https://crates.io/crates/repro)
[![CI](https://github.com/89jobrien/repro/actions/workflows/ci.yml/badge.svg)](https://github.com/89jobrien/repro/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/repro)](LICENSE)

Reproducible container image builder and OCI tarball analyzer.

Drives Docker Buildx or Podman + BuildKit to produce deterministic OCI
tarballs using `SOURCE_DATE_EPOCH` and `rewrite-timestamp=true`. Includes
a standalone analyzer that parses OCI tarballs and verifies image digests.

## Install

```bash
# From crates.io
cargo install repro

# From GitHub
cargo install --git https://github.com/89jobrien/repro.git

# From source
git clone https://github.com/89jobrien/repro.git
cd repro
cargo build --release
# Binary at target/release/repro
```

## Usage

### Build a reproducible image

```bash
# Docker (auto-detected if both runtimes are present)
repro build ./my-app -t myimage:latest -o image.tar

# Podman with rootless BuildKit
repro build ./my-app --runtime podman --rootless -t myimage:latest

# Pin timestamps to a specific date
repro build ./my-app --datetime "2024-01-01T00:00:00Z"

# Dry run — print commands without executing
repro build ./my-app --dry
```

### Analyze an OCI tarball

```bash
# Parse and display manifest structure
repro analyze image.tar

# Verify the image digest
repro analyze image.tar --expected-image-digest sha256:abc123...

# Show full manifest contents
repro analyze image.tar --show-contents
```

## Build flags

| Flag               | Description                                  | Default       |
| ------------------ | -------------------------------------------- | ------------- |
| `--runtime`        | Container runtime (`docker` or `podman`)     | Auto-detected |
| `--datetime`       | ISO 8601 timestamp for layer dates           |               |
| `--sde`            | Unix epoch for `SOURCE_DATE_EPOCH`           |               |
| `--no-cache`       | Disable build cache                          | `false`       |
| `--rootless`       | Rootless BuildKit (Podman only)              | `false`       |
| `-f, --file`       | Path to Dockerfile                           |               |
| `-o, --output`     | Output tarball path                          | `image.tar`   |
| `-t, --tag`        | Image tag                                    |               |
| `--build-arg`      | Build-time variables (`ARG=VALUE`)           |               |
| `--annotation`     | Image annotations (`KEY=VALUE`)              |               |
| `--platform`       | Target platform                              |               |
| `--buildkit-args`  | Extra BuildKit args (Podman only)            |               |
| `--buildx-args`    | Extra Buildx args (Docker only)              |               |
| `--buildkit-image` | BuildKit container image (`NAME:TAG@DIGEST`) |               |
| `--dry`            | Print commands without executing             | `false`       |

## Environment variables

CLI flags take precedence over environment variables.

| Variable                  | Equivalent flag    |
| ------------------------- | ------------------ |
| `REPRO_RUNTIME`           | `--runtime`        |
| `REPRO_DATETIME`          | `--datetime`       |
| `REPRO_SOURCE_DATE_EPOCH` | `--sde`            |
| `REPRO_CACHE`             | `--no-cache` (inv) |
| `REPRO_ROOTLESS`          | `--rootless`       |

## Architecture

Hexagonal architecture with injected dependencies for testability.

```
src/
  main.rs              CLI entry point (clap)
  lib.rs               Public API re-exports
  display.rs           Presentation layer for analyze output
  oci.rs               OCI tarball parser, digest verification
  builder/
    mod.rs             Builder orchestrator
    config.rs          BuildConfig domain type, resolution logic
    resolver.rs        Runtime detection (WhichResolver port/adapter)
    runner.rs          Command execution (ProcessRunner, DryRunner, MockRunner)
    strategy.rs        Docker/Podman build command generation
```

**Ports** (traits): `CommandRunner`, `IdempotentRunner`, `RuntimeResolver`,
`RuntimeStrategy`

**Adapters**: `ProcessRunner`, `DryRunner`, `MockRunner`, `WhichResolver`,
`MockResolver`

**Strategies**: `DockerStrategy` (docker buildx), `PodmanStrategy`
(podman run + buildctl-daemonless.sh)

## Testing

```bash
cargo test                # unit + integration tests
cargo test -- --ignored   # ignored/slow tests
```

Includes property-based tests (proptest) and formal verification stubs
(Kani). Five fuzz targets cover the OCI parser:

```bash
cargo +nightly fuzz run fuzz_parse_manifest
cargo +nightly fuzz run fuzz_snip_contents
cargo +nightly fuzz run fuzz_raw_tarball
cargo +nightly fuzz run fuzz_multi_manifest
cargo +nightly fuzz run fuzz_verify_digest
```

## License

MIT OR Apache-2.0
