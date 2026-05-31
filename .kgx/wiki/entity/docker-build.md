---
title: docker_build
source_document: src_builder
tags: [entity, function]
---

# docker_build

Docker execution path. Steps:

1. Creates a named Buildx builder instance
2. Runs `docker buildx build` with `--output type=oci,dest=<output>,rewrite-timestamp=true`
3. Injects `SOURCE_DATE_EPOCH` as `--build-arg`
4. Passes `--buildx-args` through

## Relations

- called_by → [[Builder]]
- uses_concept → [[rewrite-timestamp]], [[SOURCE-DATE-EPOCH]]
