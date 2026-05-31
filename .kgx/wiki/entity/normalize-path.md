---
title: normalize_path
source_document: src_oci
tags: [entity, function]
---

# normalize_path

Translates OCI digest references (`sha256:abc123...`) into tarball file paths
(`blobs/sha256/abc123...`). Handles the path convention used in OCI image layout.

## Relations

- called_by → [[parse-tarball]], [[parse-manifest]]
