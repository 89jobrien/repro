---
title: Builder
tags: [entity, struct, core]
---

# Builder

Resolved build configuration. Created via `Builder::new(BuildParams)` which applies
env var fallbacks and validates constraints (e.g. rootless is podman-only).

## Fields

context, runtime, rootless, buildkit_image, source_date_epoch, use_cache, file,
output, tag, build_args, annotations, platform, buildkit_args, buildx_args, dry.

## Dispatch

`build()` matches on `runtime`: "docker" -> [[docker-build]], "podman" -> [[podman-build]].

## Resolution Chain

CLI flag > env var > auto-detect. Key resolvers:

- [[resolve-runtime]]: CLI > `REPRO_RUNTIME` > `which docker/podman`
- [[resolve-sde]]: `--source-date-epoch` or `--datetime` (RFC3339, ISO)
- BuildKit image pinned by digest, `docker.io/` prefix added for podman
