---
title: AnalyzeArgs
source_document: src_main
tags: [entity, struct, clap]
---

# AnalyzeArgs

Clap-derived struct for the `analyze` subcommand:

- `tarball` — path to OCI tarball
- `expected_image_digest` — optional digest to verify
- `show_contents` — whether to print layer contents

## Relations

- consumed_by → [[parse-tarball]]
