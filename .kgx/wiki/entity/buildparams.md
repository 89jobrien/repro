---
title: BuildParams
source_document: src_builder
tags: [entity, struct]
---

# BuildParams

Intermediate struct holding raw CLI values before validation. Passed to `Builder::new()`
which resolves env var fallbacks and validates constraints.

## Relations

- resolved_by → [[Builder]]
- sourced_from → [[BuildArgs]]
