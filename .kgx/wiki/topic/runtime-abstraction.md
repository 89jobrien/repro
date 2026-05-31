---
title: Runtime Abstraction
tags: [topic, architecture]
---

# Runtime Abstraction

`repro` supports two container runtimes with a single CLI interface.

## Resolution Chain

[[resolve-runtime]] determines which runtime to use:

1. `--runtime` CLI flag (explicit)
2. `REPRO_RUNTIME` env var
3. Auto-detect on PATH (docker first, then podman)

## Execution Paths

| Runtime | Function         | Mechanism                                            |
| ------- | ---------------- | ---------------------------------------------------- |
| Docker  | [[docker-build]] | `docker buildx build` with named builder             |
| Podman  | [[podman-build]] | `podman run` + `buildctl-daemonless.sh` in container |

## Constraint Enforcement

- `--rootless` — Podman-only (error if Docker selected)
- `--buildkit-args` — Podman-only pass-through
- `--buildx-args` — Docker-only pass-through

These constraints are enforced at resolution time in [[Builder]]`::new()`.
