---
title: BuildArgs
source_document: src_main
tags: [entity, struct, clap]
---

# BuildArgs

Clap-derived struct for the `build` subcommand. Fields map to CLI flags:

- `context` — build context directory
- `runtime` — docker or podman
- `datetime` / `source_date_epoch` — timestamp control
- `file`, `output`, `tag`, `build_arg`, `annotation`, `platform`
- `dry` — print commands without executing
- `rootless` — Podman-only
- `buildkit_args` / `buildx_args` — runtime-specific pass-through

## Relations

- consumed_by → [[Builder]]
- converts_to → [[BuildParams]]
