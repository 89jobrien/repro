---
title: Repro Codebase Overview
tags: [summary, architecture]
---

# Repro Codebase Overview

Reproducible container image builder and OCI tarball analyzer. Single Rust binary
with two subcommands: `build` and `analyze`.

## Modules

- **main.rs** — CLI via clap. [[Cli]] parses into [[Commands]] enum, dispatching to
  [[builder-module]] or [[oci-module]].
- **builder.rs** — [[Builder]] struct (resolved config) created from [[BuildParams]]
  (raw CLI input). Two code paths: [[docker-build]] (buildx) and [[podman-build]]
  (buildctl-daemonless.sh). Uses [[SOURCE-DATE-EPOCH]] and [[rewrite-timestamp]] for
  determinism.
- **oci.rs** — [[parse-tarball]] does DFS from index.json. [[ManifestInfo]] is the
  primary output. [[verify-digest]] checks SHA-256 against expected value.

## Data Flow

1. CLI args -> [[BuildParams]] -> [[Builder]] (resolution + validation)
2. [[Builder]]::build() -> shell out to docker/podman -> OCI tarball on disk
3. `analyze` subcommand -> [[parse-tarball]] -> [[ManifestInfo]] tree -> display/verify
