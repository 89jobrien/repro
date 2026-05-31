---
title: verify_digest
source_document: src_oci
tags: [entity, function]
---

# verify_digest

Integrity check: compares the SHA-256 digest computed by [[parse-manifest]] against
the expected digest from the parent manifest's descriptor. Prints pass/fail status.

## Relations

- called_by → [[parse-tarball]]
