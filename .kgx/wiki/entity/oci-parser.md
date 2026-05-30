---
title: OCI Parser
tags: [entity, module]
---

# OCI Parser (oci.rs)

Parses OCI image layout tarballs. Entry point: [[parse-tarball]].

## Key Types

- [[ManifestInfo]] — parsed entry with path, contents, SHA-256 digest, media_type,
  platform, child [[ManifestDescriptor]] list
- [[PlatformSpec]] — os + architecture

## Algorithm

DFS from `index.json`. Each manifest descriptor's digest is normalized
(`sha256:abc...` -> `blobs/sha256/abc...`) and extracted from the tarball.
Handles `./` prefix variations and gzip auto-detection.

## Public API

- `parse_tarball(path)` -> `Vec<ManifestInfo>`
- `print_info(parsed, show_contents)` — display with optional truncation
- `verify_digest(parsed, expected)` — SHA-256 comparison
