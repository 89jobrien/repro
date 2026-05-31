---
title: parse_manifest
source_document: src_oci
tags: [entity, function]
---

# parse_manifest

Extracts a single manifest from the tarball:

1. Looks up path in tar entries (handles `./` prefix variations)
2. Reads content bytes
3. Computes SHA-256 digest
4. Deserializes JSON into [[ManifestInfo]]

## Relations

- called_by → [[parse-tarball]]
- produces → [[ManifestInfo]]
- calls → [[normalize-path]]
