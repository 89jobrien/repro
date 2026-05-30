# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## What This Is

Reproducible container image builder and OCI tarball analyzer. A Rust CLI that
drives Docker Buildx or Podman+BuildKit to produce deterministic OCI tarballs
using `SOURCE_DATE_EPOCH` and `rewrite-timestamp=true`.

## Commands

```bash
cargo build                # build
cargo clippy               # lint
cargo test                 # test
cargo run -- build ...     # run a reproducible build
cargo run -- analyze ...   # analyze an OCI tarball
```

## Architecture

Single-crate binary with two modules:

- `src/main.rs` — CLI entry point (clap). Defines `BuildArgs` and `AnalyzeArgs`,
  dispatches to `builder::Builder::build()` or `oci::parse_tarball()`.
- `src/builder.rs` — Build orchestration. `BuildParams` (raw CLI input) is
  resolved into `Builder` (validated config) via `Builder::new()`. Resolution
  functions handle env var fallbacks (`REPRO_RUNTIME`, `REPRO_DATETIME`,
  `REPRO_SOURCE_DATE_EPOCH`, `REPRO_CACHE`, `REPRO_ROOTLESS`). Separate code
  paths for Docker (`docker buildx`) and Podman (`podman run` +
  `buildctl-daemonless.sh`).
- `src/oci.rs` — OCI tarball parser. DFS traversal from `index.json` through
  manifest descriptors. Handles gzip auto-detection and `./` prefix variations
  in tar paths. Digest verification via SHA-256.

## Key Design Details

- `--dry` flag prints commands without executing (useful for debugging).
- BuildKit image is pinned by digest in `builder.rs` constants. Podman paths
  prepend `docker.io/` to image refs.
- `--rootless` is Podman-only; `--buildkit-args` is Podman-only;
  `--buildx-args` is Docker-only. These constraints are enforced at resolution
  time.
- Env vars serve as defaults for most CLI flags — CLI args take precedence.
