---
title: rewrite-timestamp
source_document: src_builder
tags: [entity, concept, reproducibility]
---

# rewrite-timestamp

A BuildKit output option (`rewrite-timestamp=true`) that rewrites all file timestamps
in OCI image layers to SOURCE_DATE_EPOCH. This is the mechanism that makes builds
reproducible — without it, timestamps from the build host leak into layers.

## Relations

- used_by → [[docker-build]], [[podman-build]]
- works_with → [[SOURCE-DATE-EPOCH]]
