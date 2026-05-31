---
title: podman_build
source_document: src_builder
tags: [entity, function]
---

# podman_build

Podman execution path. Steps:

1. Runs `podman run` with the pinned BuildKit image
2. Entrypoint: `buildctl-daemonless.sh`
3. Mounts build context and output directory
4. Passes `--buildkit-args` through
5. Supports `--rootless` mode

## Relations

- called_by → [[Builder]]
- uses_concept → [[rewrite-timestamp]], [[SOURCE-DATE-EPOCH]]
