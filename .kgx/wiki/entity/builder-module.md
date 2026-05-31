---
title: builder module
source_document: src_main
tags: [entity, module]
---

# builder module

`src/builder.rs` — contains all build orchestration logic:

- [[BuildParams]] — raw input
- [[Builder]] — resolved config
- [[resolve-runtime]], [[resolve-sde]] — resolution functions
- [[docker-build]], [[podman-build]] — execution paths
