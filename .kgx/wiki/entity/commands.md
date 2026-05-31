---
title: Commands
source_document: src_main
tags: [entity, enum, clap]
---

# Commands

Clap subcommand enum with two variants:

- `Build(BuildArgs)` — triggers [[docker-build]] or [[podman-build]] via [[Builder]]
- `Analyze(AnalyzeArgs)` — triggers [[parse-tarball]]

## Relations

- variant_contains → [[BuildArgs]]
- variant_contains → [[AnalyzeArgs]]
