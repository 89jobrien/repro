# Conformance Spec — Hexagonal Port/Adapter Contracts

## Overview

This spec defines the behavioral contracts for each port trait in the `repro`
crate. Conformance tests verify that every adapter satisfies these contracts.

---

## Port 1: `CommandRunner`

**Trait:** `builder::CommandRunner`

**Contract:**
- C1.1: `run(&[String])` returns `Ok(())` when the command succeeds.
- C1.2: `run(&[String])` returns `Err` when the command fails (non-zero exit).
- C1.3: The command receives all args passed (no arg mutation/dropping).

**Sub-port:** `IdempotentRunner` (extends `CommandRunner`)
- C1.4: `run_no_check(&[String])` never returns an error (void return).
- C1.5: `run_no_check` still executes the command (observable side effect).

**Adapters under test:**
- `ProcessRunner` — real process execution
- `DryRunner` — logs without executing
- `MockRunner` — captures for assertions

---

## Port 2: `RuntimeResolver`

**Trait:** `builder::RuntimeResolver`

**Contract:**
- C2.1: `resolve(name)` returns `ResolvedRuntime` with `name` matching input.
- C2.2: `resolve(name)` returns a `path` that is absolute.
- C2.3: `resolve(name)` returns `Err` when the binary does not exist.

**Adapters under test:**
- `WhichResolver` — PATH-based lookup
- `MockResolver` — fixed path

---

## Port 3: `RuntimeStrategy`

**Trait:** `builder::RuntimeStrategy`

**Contract:**
- C3.1: `build_commands(config)` returns at least one `CommandSpec`.
- C3.2: Every `CommandSpec.args` is non-empty (has a program name).
- C3.3: The generated commands include `SOURCE_DATE_EPOCH=<value>` somewhere
         in args (reproducibility guarantee).
- C3.4: The generated commands include `rewrite-timestamp=true` in an output
         option (reproducibility guarantee).
- C3.5: If `config.tag` is set, the tag appears in the generated commands.
- C3.6: If `config.build_args` is non-empty, each appears in the commands.

**Adapters under test:**
- `DockerStrategy` — Docker Buildx commands
- `PodmanStrategy` — Podman + buildctl commands

---

## Port 4: Integration — `Builder::with_deps`

**Contract:**
- C4.1: `Builder::with_deps(params, resolver, runner).build()` invokes the
         runner with the commands from the strategy.
- C4.2: The runner receives commands in order (create before build for Docker).
- C4.3: If the runner returns `Err`, `build()` propagates the error.
- C4.4: Invalid runtime in config returns `Err` from `build()`.
