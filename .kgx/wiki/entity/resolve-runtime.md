---
title: resolve_runtime
source_document: src_builder
tags: [entity, function]
---

# resolve_runtime

Resolution priority:

1. CLI `--runtime` flag
2. `REPRO_RUNTIME` environment variable
3. Auto-detect: checks PATH for `docker` then `podman`

Returns absolute path to the runtime binary.

## Relations

- called_by → [[Builder]]
