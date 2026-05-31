---
title: Builder
source_document: src_builder
tags: [entity, struct, core]
---

# Builder

Central orchestration struct. Created via `Builder::new(params)` which:

1. Resolves runtime via [[resolve-runtime]]
2. Resolves SOURCE_DATE_EPOCH via [[resolve-sde]]
3. Validates Podman-only / Docker-only constraints

Exposes `build()` method that dispatches to [[docker-build]] or [[podman-build]].

## Fields

- `context`, `runtime`, `rootless`, `buildkit_image`
- `source_date_epoch`, `file`, `output`, `tag`
- `build_arg`, `annotation`, `platform`
- `dry`, `buildkit_args`, `buildx_args`

## Relations

- resolves_from → [[BuildParams]]
- calls → [[docker-build]], [[podman-build]]
- uses → [[resolve-runtime]], [[resolve-sde]]
- uses_concept → [[SOURCE-DATE-EPOCH]], [[rewrite-timestamp]]
