---
title: SOURCE_DATE_EPOCH
source_document: src_builder
tags: [entity, concept, reproducibility]
---

# SOURCE_DATE_EPOCH

A reproducible-builds standard. A unix timestamp that, when injected into the build,
ensures all file modification times in the resulting image are clamped to this value.

Used together with [[rewrite-timestamp]] to produce bit-for-bit identical OCI images
across builds.

## Env Vars

- `REPRO_SOURCE_DATE_EPOCH` — raw timestamp
- `REPRO_DATETIME` — human-readable datetime (parsed by [[resolve-sde]])

## Relations

- resolved_by → [[resolve-sde]]
- used_by → [[docker-build]], [[podman-build]]
