---
title: oci module
source_document: src_main
tags: [entity, module]
---

# oci module

`src/oci.rs` — contains all OCI tarball analysis logic:

- [[parse-tarball]] — entry point
- [[parse-manifest]] — single manifest extraction
- [[verify-digest]] — integrity check
- [[normalize-path]] — digest-to-path conversion
- [[ManifestInfo]], [[ManifestDescriptor]], [[PlatformSpec]] — data structures
