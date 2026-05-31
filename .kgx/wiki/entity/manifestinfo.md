---
title: ManifestInfo
source_document: src_oci
tags: [entity, struct]
---

# ManifestInfo

Deserialized manifest data. Fields:

- `path` — location in tarball
- `contents` — raw JSON bytes
- `digest` — computed SHA-256
- `media_type` — OCI media type string
- `platform` — optional [[PlatformSpec]]
- `manifests` — child [[ManifestDescriptor]] list (for index manifests)

## Relations

- produced_by → [[parse-manifest]]
- contains → [[ManifestDescriptor]], [[PlatformSpec]]
