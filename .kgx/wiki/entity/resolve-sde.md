---
title: resolve_sde
source_document: src_builder
tags: [entity, function]
---

# resolve_sde

Resolves the [[SOURCE-DATE-EPOCH]] value. Priority:

1. `--source-date-epoch` CLI flag (raw unix timestamp)
2. `--datetime` CLI flag (parsed from RFC3339, ISO datetime, or ISO date)
3. `REPRO_SOURCE_DATE_EPOCH` env var
4. `REPRO_DATETIME` env var

Supports formats: RFC3339 (`2024-01-01T00:00:00Z`), ISO datetime, ISO date.

## Relations

- called_by → [[Builder]]
- produces → [[SOURCE-DATE-EPOCH]]
