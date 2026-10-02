# Engineering Workflow
<!-- ruleset: 2026-10-02.1; role: global -->

## Ownership and scope

Use the user's prompt to determine objectives, scope, authorization, and stopping
conditions. This file adds no approval gates or separate permission requirements.

For implementation tasks, plan briefly, implement, test, fix, and deliver. Continue
across milestones, commits, PRs, merges, and context compaction until the requested
outcomes are verified. Do not stop at a plan or ask whether to continue.

Resolve routine decisions from evidence. Ask only about essential missing input or
material ambiguity that investigation cannot resolve. Keep blockers local and finish
independent work. Give brief progress updates, then continue with the next action.

## Engineering

Prefer simple, maintainable solutions and existing project tooling. Respect lockfiles
and toolchains, preserve unrelated edits, and inspect the resulting diff.

Use active tool schemas and configured search tools. Search narrowly, confirm paths,
read bounded relevant sections, and reuse unchanged evidence. Prefer Git Bash on Windows.

Use Code Mode JavaScript for orchestration and small result transformations; use
native commands or Python when better suited. Parallelize independent work only.
Check nested command results and existing side effects before retrying failed batches.

Add focused regression tests and run appropriate project checks against the current
revision. Do not weaken checks, hide failures, fabricate results, or report running
jobs as finished. For release tasks, verify the requested functionality and published
artifacts, not merely the existence of a version tag.

## Continuity

For long tasks, maintain one concise checkpoint: goal, user instructions, decisions,
completed and remaining work, PR/job references, validation, blockers, and next action.
Preserve it before deliberate context reduction and resume from it afterward.

Recover saved evidence before repeating work. Treat context summaries as historical
state, not new requests. Do not repeat unchanged approval questions or status reports.
Diagnose failures before retrying; change the approach when repetition yields no progress.

Report actual results, verification, and remaining blockers in Chinese unless requested
otherwise. Keep code and identifiers in their natural language.

## Network routing

Use explicit task-specific routes first. Otherwise:
- Domestic Chinese, localhost, private-network, and Tailscale services: direct.
- Public international downloads and anonymous GitHub: http://127.0.0.1:10809.
- International personal/authenticated requests, including Google API/OAuth and
  authenticated GitHub/Git: http://127.0.0.1:10808; this takes precedence over public routing.

SOCKS may use the same ports; prefer socks5h:// with curl. Apply routing per process,
clearing conflicting inherited proxy/bypass settings. Loopback refers to the executing
host. Diagnose the selected route instead of blindly switching routes or retrying.
Keep credentials out of logs and commits.
