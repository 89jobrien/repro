---
title: parse_tarball
source_document: src_oci
tags: [entity, function, core]
---

# parse_tarball

Entry point for OCI analysis. Algorithm:

1. Opens tarball, reads `index.json`
2. DFS traversal through [[ManifestDescriptor]] references
3. Calls [[parse-manifest]] for each descriptor
4. Calls [[verify-digest]] to validate integrity
5. Prints tree structure with platform info

## Relations

- calls → [[parse-manifest]], [[verify-digest]], [[normalize-path]]
- reads → [[ManifestInfo]], [[ManifestDescriptor]]
