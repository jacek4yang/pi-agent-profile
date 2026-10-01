# Architecture

One Rust binary embeds AGENTS.md and APPEND_SYSTEM.md and generates JSON from typed,
validated selections. There is no Node/Python installer runtime, downloaded config
script, background daemon, global permission bypass, or automatic login.

- `config`: answers schema, six-package catalogue, field-level merge and rule rendering.
- `storage`: pure plan, managed rule blocks, bounded paths, snapshot/hash/journal recovery.
- `system`: executable discovery, Git Bash preference, controlled subprocess routing,
  timeout/cancellation and honest diagnostics.
- `main`: interactive questions, explicit confirmation and automation entry points.

Settings JSON is recursively merged. Unknown keys and unrelated package entries are
preserved. Package identity matches Pi's npm package-name convention; filters survive
unpinning. Conflicting duplicate declarations fail rather than dropping information.
A declined installed plugin can be kept unmanaged or detached from global loading.
Caches and project-specific package declarations are not deleted.

Rules use one managed block in each file. Bytes outside that block remain user-owned;
edits inside it require review and explicit --overwrite-managed. The exact previously
supplied rule templates can be adopted without duplicating them. Project approval
rules and project APPEND_SYSTEM.md precedence remain unchanged.

Saved choices are at `<agent-dir>/.pi-profile/state.json`. Re-running shows a menu;
update uses saved choices; configure asks again with previous choices as defaults.
A semantically unchanged apply creates no new snapshot. Disabling Code Mode removes
its tool selection rather than writing a nonexistent `codemode.mode=off` setting.

`tuning=false`, `model=null`, `provider_proxy=null`, and `steering_all=false` mean
leave corresponding current values alone; they are not a reset-to-factory command.
`provider_proxy=""` explicitly removes httpProxy. Package `disable` explicitly removes
the global declaration, whereas `keep` leaves it untouched. Turning rules off removes
only the managed block. Thus optional features have explicit, documented semantics.

No provider/model availability or actual language-server operation is inferred from
configuration. Doctor separates file presence from real runtime acceptance. Upstream
model IDs, toolchains, APIs and extension schemas still require validation on the host.

## Release gates

On trusted main, the source job aligns workspace package versions in Cargo.lock
without deliberately upgrading locked dependencies, applies rustfmt, and records
the resulting commit before downstream tests. PRs do not receive automatic writes.
Five native platform builds and three real-Pi integration jobs gate publication.
Integration runs in a disposable directory without credentials, verifies missing
fixed-version packages, retained resource filters, repeat updates and detach without
cache deletion. Pinned versions use explicit install when missing or changed because
Pi's update skips exact npm pins. Existing release assets are never overwritten.
