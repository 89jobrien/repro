---
title: Reproducibility Mechanism
tags: [topic, architecture, reproducibility]
---

# Reproducibility Mechanism

The core value proposition of `repro`: given the same source and the same timestamp,
produce bit-for-bit identical OCI tarballs.

## How It Works

1. User supplies a timestamp via `--datetime`, `--source-date-epoch`, or env vars
2. [[resolve-sde]] normalizes it to a unix timestamp
3. The timestamp is injected as `SOURCE_DATE_EPOCH` build-arg
4. BuildKit's `rewrite-timestamp=true` output option clamps all file mtimes in layers
5. The resulting OCI tarball has deterministic content — same inputs = same SHA-256

## Key Entities

- [[SOURCE-DATE-EPOCH]] — the timestamp standard
- [[rewrite-timestamp]] — the BuildKit mechanism
- [[resolve-sde]] — resolution logic
- [[Builder]] — orchestrates the flow

## Verification

The `analyze` subcommand ([[parse-tarball]]) can verify a tarball's digest matches
expectations, confirming reproducibility was achieved.
