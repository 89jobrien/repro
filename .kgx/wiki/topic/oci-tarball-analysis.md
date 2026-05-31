---
title: OCI Tarball Analysis
tags: [topic, architecture]
---

# OCI Tarball Analysis

The `analyze` subcommand provides structural inspection and integrity verification
of OCI tarballs.

## Algorithm

[[parse-tarball]] performs DFS traversal:

1. Read `index.json` from tarball root
2. For each [[ManifestDescriptor]], resolve digest → path via [[normalize-path]]
3. Extract and parse via [[parse-manifest]]
4. Verify integrity via [[verify-digest]]
5. Recurse into child manifests

## Handling Quirks

- Gzip auto-detection for compressed layers
- `./` prefix variations in tar entry paths
- Multi-arch images via [[PlatformSpec]] in descriptors

## Output

Prints a tree of manifests with:

- Platform info (os/arch)
- Digest verification status (pass/fail)
- Optional layer contents listing (`--show-contents`)
