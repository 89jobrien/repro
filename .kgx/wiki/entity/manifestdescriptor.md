---
title: ManifestDescriptor
source_document: src_oci
tags: [entity, struct]
---

# ManifestDescriptor

A reference to a child manifest within an OCI index. Fields:

- `digest` — content-addressable reference (e.g. `sha256:...`)
- `mediaType` — manifest media type
- `platform` — optional [[PlatformSpec]]

Used by [[parse-tarball]] to drive DFS traversal.

## Relations

- contained_in → [[ManifestInfo]]
- references → [[PlatformSpec]]
