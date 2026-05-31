---
title: PlatformSpec
source_document: src_oci
tags: [entity, struct]
---

# PlatformSpec

Describes the target platform for a manifest:

- `os` — e.g. `linux`
- `architecture` — e.g. `amd64`, `arm64`

Used in multi-arch OCI images to select the correct manifest.

## Relations

- contained_in → [[ManifestDescriptor]], [[ManifestInfo]]
