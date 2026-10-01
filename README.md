# Pi Agent Profile

A portable, idempotent workstation profile for [Pi](https://github.com/earendil-works/pi), designed to turn a fresh or partially configured Pi environment into the same high-capability coding setup on Linux, macOS, and Windows.

Repository: `https://github.com/jacek4yang/pi-agent-profile`

This repository is intentionally **not** a backup of `~/.pi/agent`. It is a declarative profile: it declares the global instructions, Pi settings, extension configuration, required packages, network policy, and reconciliation behavior that should exist on a machine. The bootstrap detects what is already present, updates or installs what is missing, preserves unrelated settings, backs up managed files, applies the profile, and runs non-destructive checks.

## Goals

- One profile for Linux, macOS, and Windows.
- Prefer Git Bash on Windows so the same Bash workflow is used everywhere.
- Work with an existing Pi installation, even when only some extensions are installed.
- Update Pi and all managed Pi packages by default.
- Preserve unrelated user configuration instead of replacing `settings.json` wholesale.
- Keep authentication, sessions, secrets, caches, and machine-local state out of Git.
- Make long-running coding agents more autonomous without bypassing real authorization boundaries.
- Keep Code Mode, context pruning, compaction, LSP, search, validation, and network routing consistent across machines.
- Make every profile application recoverable through automatic backups.

## Managed Pi packages

The profile currently manages these package sources:

| Package | Role |
| --- | --- |
| `@ff-labs/pi-fff` | Fast file/search workflow used by the agent |
| `@narumitw/pi-lsp` | LSP integration |
| `pi-context-usage` | Context usage visibility |
| `pi-context-prune` | Agent-controlled context pruning |
| `pi-web-search` | Web search extension |
| `@lll9p/pi-better-compaction` | Improved compaction pipeline |

Package specs are intentionally unpinned. A normal online bootstrap asks Pi to update each managed source, so a machine with stale packages converges toward current releases. If reproducibility is more important than automatic upgrades, pin package versions in `profile.json` on a release/tagged branch.

## What the profile manages

The managed state is declared in `profile.json`.

```text
AGENTS.md                                      -> ~/.pi/agent/AGENTS.md
APPEND_SYSTEM.md                               -> ~/.pi/agent/APPEND_SYSTEM.md
config/settings.patch.json                     -> merge into settings.json
config/pi-fff.json                             -> merge into pi-fff.json
config/lsp.json                                -> merge into lsp.json
config/context-prune.json                      -> merge into context-prune/settings.json
config/better-compaction.json                  -> merge into extensions/pi-better-compaction/config.json
```

Text policy files are managed as complete files. JSON configuration is deep-merged: keys owned by the profile are reconciled while unrelated keys already present on the machine are retained.

The default profile includes:

- OpenAI as the default provider and `gpt-6-astra` at medium thinking.
- Code Mode as the default tool surface, with `mode: "only"`.
- bounded retry/timeout behavior;
- native Pi compaction settings and model-specific token budgets;
- `pi-context-prune` in `agentic-auto` mode using `gpt-6-luna` for summaries;
- `pi-better-compaction` with continuity protection enabled;
- FFF configuration that avoids broad root/home scanning;
- LSP definitions for `ty`, `ruff`, and `rust-analyzer`;
- global engineering rules for autonomous execution, evidence, validation, context continuity, and network routing.

## Deliberately not synchronized

The following must remain machine-local and are never part of this public profile:

- `auth.json` and authentication state;
- OAuth tokens and API keys;
- browser cookies or signed URLs;
- session JSONL files and conversation history;
- provider payload/debug dumps;
- Pi package caches and installed package directories;
- generated artifacts and compaction debug data;
- project-local `.pi/`, project `AGENTS.md`, or project approval rules;
- machine-specific secrets;
- `config/local.json`.

Do not commit those files even if the repository is later made private.

## Prerequisites

For an existing Pi environment:

- Node.js `>= 22.19.0`;
- Pi available on `PATH`;
- Git for cloning/updating the repository.

If Pi is missing but `npm` is available, an online bootstrap can install the official `@earendil-works/pi-coding-agent` package.

Recommended development tools, checked by `doctor` but not forcibly installed by this profile:

- `git`
- `rg`
- `fd`
- `uv`
- `ruff`
- `ty`
- `rustup`
- `rust-analyzer`

Language toolchains stay project-owned. For example, a Rust project may need `rust-analyzer` installed for the exact rustup toolchain selected by that project.

## Network policy

This profile follows three routing classes. The routes are also documented in the global `AGENTS.md` so Pi knows how to classify task traffic.

| Traffic | Route |
| --- | --- |
| Known domestic Chinese services, localhost, private/LAN/Tailscale resources | Direct unless an endpoint-specific rule says otherwise |
| Public international resources, anonymous GitHub, public packages/artifacts | `http://127.0.0.1:10809` (or SOCKS on the same port when appropriate) |
| Requests carrying personal credentials/private account data, authenticated GitHub/Git, Google API/OAuth flows | `http://127.0.0.1:10808` (or SOCKS on the same port when appropriate) |

Authentication-sensitive routing takes precedence over the public-download route.

The bootstrap itself uses the **public** proxy for Pi/npm/package update traffic. If `10809` is unavailable, it does not silently rotate to `10808` or direct Internet access: online updates are skipped and local profile application can still continue.

`127.0.0.1` means the host actually executing the command. Do not assume those listeners exist inside an SSH target, container, CI runner, or remote tool host.

## Quick start

### Linux

Clone the public profile through the public international route:

```bash
git -c http.proxy=http://127.0.0.1:10809 \
  clone https://github.com/jacek4yang/pi-agent-profile.git
cd pi-agent-profile
bash bootstrap.sh
```

### macOS

The same flow applies:

```bash
git -c http.proxy=http://127.0.0.1:10809 \
  clone https://github.com/jacek4yang/pi-agent-profile.git
cd pi-agent-profile
bash bootstrap.sh
```

### Windows (preferred: Git Bash)

Git for Windows / Git Bash is the preferred path. From PowerShell:

```powershell
git -c http.proxy=http://127.0.0.1:10809 clone https://github.com/jacek4yang/pi-agent-profile.git
cd pi-agent-profile
.\bootstrap.ps1
```

`bootstrap.ps1` searches the normal Git for Windows locations and the installed `git.exe`. If Git Bash exists, it delegates to the same `bootstrap.sh` used on Linux/macOS. Only when Git Bash is unavailable does it fall back to the shared Node core directly.

If you are already inside Git Bash:

```bash
bash bootstrap.sh
```

The repository intentionally does not hard-code `/bin/bash` into Pi's portable `settings.json`.

## What `bootstrap` does

A normal online run is idempotent and performs this sequence:

1. Load and validate `profile.json`.
2. Verify the minimum Node.js version.
3. Locate Pi; install the official Pi npm package if Pi is missing and online installation is possible.
4. Check the configured public proxy before attempting public Internet updates.
5. Run `pi update --self` unless disabled.
6. Install missing managed Pi packages.
7. Ask Pi to update every managed package source.
8. Refuse to shadow an existing global `AGENTS.override.md` silently.
9. Create a timestamped backup of every managed destination.
10. Deep-merge managed JSON configuration and install the global instruction files.
11. Apply the optional machine-local `config/local.json` overlay when present.
12. Write `.profile-state.json` in the Pi agent directory.
13. Run the profile doctor.
14. Tell you to restart Pi or run `/reload` in already-running Pi sessions.

Existing unrelated `settings.json` keys and package declarations are preserved unless the profile explicitly owns the same key.

## Common commands

Linux/macOS/Git Bash:

```bash
# Normal reconciliation: update Pi/packages, back up, apply profile, doctor
bash bootstrap.sh

# Keep the installed Pi core version, but still reconcile packages/profile
bash bootstrap.sh --no-self-update

# No network operations; apply only local profile files/config
bash bootstrap.sh --offline

# Preview writes/commands without modifying the Pi profile
bash bootstrap.sh --dry-run

# Non-destructive environment/profile inspection
bash doctor.sh

# Treat profile-level doctor problems as a failing exit status
bash doctor.sh --strict

# Restore the newest automatically created profile backup
bash restore.sh

# Restore a specific backup
bash restore.sh ~/.pi/agent/.profile-backups/<timestamp>
```

Windows PowerShell entry points:

```powershell
.\bootstrap.ps1
.\bootstrap.ps1 --offline
.\bootstrap.ps1 --no-self-update
.\doctor.ps1
```

For restore on Windows, use Git Bash and `bash restore.sh` so restore behavior stays identical across platforms.

## First deployment onto an existing Pi installation

No uninstall is required. The intended workflow is:

```bash
cd pi-agent-profile
bash doctor.sh
bash bootstrap.sh --dry-run
bash bootstrap.sh
```

This is specifically designed for machines where:

- some managed extensions are already installed;
- some extensions are missing;
- installed extensions are stale;
- `settings.json` contains additional unrelated configuration;
- old Pi sessions already exist.

The profile does not delete sessions, credentials, unrelated extensions, or unknown settings.

After apply:

```text
/reload
```

Run `/reload` inside each Pi session that was already open. New Pi sessions automatically load the new global files.

## Machine-local provider proxy (`10810`)

The portable profile does **not** force Pi's own model/provider transport through port `10810`, because that listener may not exist on every machine.

On a machine that does use the same provider proxy:

```bash
cp config/local.example.json config/local.json
bash bootstrap.sh --offline
```

`config/local.example.json` contains:

```json
{
  "settingsPatch": {
    "httpProxy": "http://127.0.0.1:10810"
  }
}
```

`config/local.json` is gitignored. It should hold non-secret machine-local overrides only.

## Updating this profile later

For this public repository, pull through the public route:

```bash
cd pi-agent-profile
git -c http.proxy=http://127.0.0.1:10809 pull --ff-only
bash bootstrap.sh
```

If the repository is ever private or the Git operation is authenticated, use the private/authenticated route instead:

```bash
git -c http.proxy=http://127.0.0.1:10808 pull --ff-only
bash bootstrap.sh
```

A profile update and a Pi/plugin update are deliberately coupled by the normal bootstrap. Use `--offline` or `--no-self-update` when you intentionally want a narrower change.

## Doctor output

`doctor` checks profile-level state without reading secret contents. It reports:

- Node.js minimum-version compliance;
- whether Pi is on `PATH`;
- whether `AGENTS.override.md` would shadow the managed global rules;
- whether all managed files exist;
- whether managed package sources are configured;
- whether legacy `pi-lsp.json` still exists;
- whether `auth.json` exists **without reading or printing it**;
- availability of common development commands;
- whether the configured `10808`, `10809`, and example `10810` listeners are reachable.

`doctor.sh --strict` exits non-zero for profile-level problems, which is useful for CI or workstation audits.

## Backups and rollback

Every non-dry-run apply creates a backup under:

```text
~/.pi/agent/.profile-backups/<timestamp>/
```

The backup contains a manifest recording which managed files existed before the apply. Restore therefore handles both cases:

- pre-existing managed files are copied back;
- managed files that did not exist before the apply are removed.

Restore does not attempt to downgrade Pi itself or npm package installations; it restores profile-managed files/configuration. If a package update must also be rolled back, use a version-pinned package source or a known repository release.

## Global instruction files

### `AGENTS.md`

Contains durable engineering behavior:

- evidence-first execution;
- end-to-end autonomous completion within already authorized scope;
- minimal and safe clarification behavior;
- narrow search/read/edit strategy;
- explicit command exit-status and side-effect handling;
- validation and acceptance criteria;
- durable checkpoints for long tasks;
- blocker scoping instead of treating one permission/network failure as a whole-task failure;
- prescribed proxy routing and secret-safe diagnostics.

It explicitly avoids equating autonomy with unlimited permission. Project approval gates, destructive production actions, releases, merges, and other authorization boundaries remain authoritative.

### `APPEND_SYSTEM.md`

Contains Pi runtime/tool guidance that should not be duplicated throughout the durable engineering policy:

- Code Mode orchestration principles;
- QuickJS vs Node.js boundaries;
- when JavaScript is preferable to spawning Python/Node and when it is not;
- context-prune behavior;
- recovery of pruned evidence;
- compaction continuity constraints;
- Git Bash preference on native Windows.

The correct Pi filename is `APPEND_SYSTEM.md`.

## LSP behavior

The shared LSP profile defines:

- `.py` / `.pyi` -> `ty`;
- `.py` / `.pyi` -> `ruff`;
- `.rs` -> `rust-analyzer`.

The profile does not force-install project toolchains. If a Rust repository pins a rustup toolchain, make sure `rust-analyzer` is available for that exact toolchain, for example:

```bash
rustup show active-toolchain
rustup component add rust-analyzer rust-src --toolchain <toolchain>
```

A global `rust-analyzer --version` succeeding is not proof that the project-selected toolchain can launch its language server.

## Legacy configuration

Older machines may still have:

```text
~/.pi/agent/pi-lsp.json
```

The current profile manages:

```text
~/.pi/agent/lsp.json
```

`doctor` warns about the legacy file but does not delete it automatically. Remove/migrate it only after verifying which installed LSP extension version is active and that the new configuration is working.

## Repository layout

```text
.
├── AGENTS.md
├── APPEND_SYSTEM.md
├── LICENSE
├── README.md
├── SHA256SUMS
├── bootstrap.ps1
├── bootstrap.sh
├── doctor.ps1
├── doctor.sh
├── profile.json
├── restore.sh
├── config/
│   ├── better-compaction.json
│   ├── context-prune.json
│   ├── local.example.json
│   ├── lsp.json
│   ├── pi-fff.json
│   └── settings.patch.json
├── scripts/
│   └── profile.mjs
├── docs/
│   ├── DESIGN.md
│   └── SECURITY.md
└── .github/
    └── workflows/
        └── ci.yml
```

## Configuration ownership

`profile.json` is the single manifest for managed packages/files and network defaults.

`config/settings.patch.json` deliberately contains only portable settings. Machine-specific provider routing belongs in ignored `config/local.json`.

When adding a new managed JSON file, add it to `profile.json` and decide explicitly whether the merge semantics are correct. Arrays in profile-owned fields are replaced; nested objects are recursively merged.

When adding another Pi package, add its package source to `profile.json`. Do not commit the resulting `~/.pi/agent/npm/` package tree.

## Security model

This repository is safe to keep public **only if secrets remain excluded**.

Before every push, verify that the staged diff contains no:

- API keys or tokens;
- OAuth refresh/access tokens;
- `auth.json`;
- session logs;
- Authorization headers;
- signed URLs;
- cookies;
- private provider payloads;
- machine-specific secrets.

The profile never needs those values in Git. Authentication is performed independently on each workstation through Pi's supported login/credential mechanisms.

See [`docs/SECURITY.md`](docs/SECURITY.md) for the explicit security boundary.

## CI

GitHub Actions validates the profile on Linux, macOS, and Windows. It checks:

- Node syntax;
- JSON parsing;
- shell-script syntax where applicable;
- repository SHA-256 manifest integrity;
- offline reconciliation against an isolated temporary Pi agent directory;
- preservation of an unrelated existing setting;
- strict profile doctor behavior with a stub Pi executable.

The CI does not use real credentials, real Pi sessions, or your local proxies.

## Troubleshooting

### Public proxy `10809` is not listening

Online Pi/package updates are skipped rather than trying another route automatically. Start the intended local proxy, or intentionally run:

```bash
bash bootstrap.sh --offline
```

### `AGENTS.override.md` exists

A global `AGENTS.override.md` shadows `AGENTS.md`. Bootstrap refuses to pretend the managed rules are active. Inspect the override and intentionally remove/rename/merge it before applying the profile.

### A managed JSON file is invalid

Bootstrap stops instead of overwriting malformed JSON. Repair the file or restore a known backup first.

### A plugin is configured but behaves incorrectly

Run:

```bash
pi list
bash doctor.sh
```

Then update through the normal bootstrap. Avoid copying plugin cache directories between machines.

### Pi was already running during apply

The files are on disk, but the existing session may still have the old prompt/configuration. Run:

```text
/reload
```

or restart Pi.

### Windows does not use Git Bash

Install Git for Windows or ensure its `git.exe`/`bash.exe` is discoverable. `bootstrap.ps1` intentionally prefers Git Bash when available.

## Reproducible snapshots

The normal profile tracks current Pi/plugin releases. For a frozen workstation baseline:

1. create a Git tag for the profile;
2. pin required npm package versions in `profile.json`;
3. keep `SHA256SUMS` updated;
4. deploy from that tag;
5. use `--no-self-update` when the Pi core itself must remain fixed.

This separates the everyday "keep my Pi environment current" workflow from a strict reproducible release workflow.

## License

MIT. See [`LICENSE`](LICENSE).
