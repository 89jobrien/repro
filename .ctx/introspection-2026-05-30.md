# Introspect Report — 2026-05-30

## Blocking (breaks agent execution)

- [using-godmode:67] `godmode:agent-governance` listed in skill table but no
  `skills/agent-governance/` directory or SKILL.md exists. Skill invocation will fail.
- [using-godmode:63] `godmode:context-map` listed in skill table but no
  `skills/context-map/` directory or SKILL.md exists. Skill invocation will fail.
- [using-godmode:65] `godmode:doublecheck` listed in skill table but no
  `skills/doublecheck/` directory or SKILL.md exists. Skill invocation will fail.
- [using-godmode:62] `godmode:rust-conventions` listed in skill table but no
  `skills/rust-conventions/` directory or SKILL.md exists. Skill invocation will fail.
- [skill-index.md:33] Same 4 skills listed in skill-index.md with no backing SKILL.md.

## Suggestion (degrades reliability)

- [using-godmode:123] CLI quick reference says `godmode agent dispatch <path> [--max N]`
  but task-management/references/godmode-cli.md says `godmode agent <plan.md> [--max 5]`.
  Inconsistent subcommand name — one of them is wrong.
- [using-godmode:107-124] CLI quick reference is missing `godmode status`,
  `godmode task remove`, `godmode task clear`, `godmode task pull`,
  `godmode task push-done` which all appear in
  task-management/references/godmode-cli.md. Agents using the quick reference
  won't know these commands exist.
- [using-godmode/helpers/session-start.sh:23] References `godmode status` which
  is not in the using-godmode quick reference (though it is in the full CLI ref).

## Nitpick (cosmetic or minor)

- [mini-context-graph:67,91] Uses `cat <<'JSON'` and `cat <<'MD'` as bash
  heredocs piped to `kgx`. Acceptable for CLI examples but inconsistent with
  the nu-first convention. Consider documenting nu equivalents.

## No issues found

- Merge strategy: `--no-ff` consistent across parallel-agents, wave-integration,
  merge, tackle-issues. No cherry-pick for parallel agents.
- Concurrency cap: `5` consistent across using-godmode, task-management,
  tackle-issues.
- BLOCKED.md trigger: `3 failed attempts` consistent across parallel-agents,
  tackle-issues.
- `--no-verify`: correctly prohibited in cap, merge, ci-fix, tackle-issues,
  using-godmode.
- All `references/` and `helpers/` files referenced in SKILL.md files exist on
  disk (verified via Glob).
- No bare `op://` URIs, no `gh run watch`, no `cd && git` anti-patterns found
  in any SKILL.md.
