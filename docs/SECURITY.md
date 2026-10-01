# Security and recovery boundaries

The installer never asks for API keys, imports auth.json, logs provider payloads,
or synchronizes sessions. Saved choices contain models, flags and credential-free
proxy URLs, not authentication. Login remains a separate Pi operation.

Public package commands use a temporary neutral working directory and empty npm
user/global configuration files, strip common token/API-key/preload environment
variables, and explicitly set npm registry and proxy variables. This is not a
sandbox for installed extensions: third-party Pi packages can execute code with
the user's privileges. Review the packages before approving installation. A
custom npmCommand is refused for online synchronization rather than guessing its
routing. Private npm registries are intentionally outside this bootstrap's scope.

Split mode never retries over the private proxy or direct route. The private route
is included in generated instructions for later authenticated agent tasks; the
installer does not log in or send OAuth/API requests. HTTP proxy listener reachability
is not proof of end-to-end Internet access. Direct and inherit are explicit user
choices, not automatic fallbacks. No system-wide proxy or firewall changes are made.

Only eight known configuration paths are mutable. Existing symlinks inside the
agent directory are rejected; archives do not supply arbitrary write paths.
The installer serializes its writers with an OS file lock, validates every target
before apply, and detects changes between planning and writes. It does not share
Pi's own configuration lock: close active Pi processes during reconfiguration.
Protection against a hostile same-account process racing filesystem calls is not
claimed. Keep the agent directory in your own account, not a shared writable folder.

Each changed file is atomically replaced, but a multi-file operation is not a single
filesystem transaction. A checksum-verified snapshot and pending journal make it
recoverable after a crash. An interrupted write blocks further changes until restore.
Restore preflights the whole snapshot, verifies hashes and target paths, refuses
unrelated later edits unless --force is explicitly supplied, and snapshots current
files before restoring. Force does not bypass backup integrity checks.

Backups may contain secrets that you previously put directly into settings.json.
They remain local, with owner-only permissions on Unix and normal user-directory ACL
inheritance on Windows. They are not encrypted and must not be published. Old
snapshots are not automatically removed. Windows users should keep their profile
under their private user directory. Credential files and sessions are never backed up.

Pi/plugin updates are external package-manager operations. Configuration rollback
cannot undo their executable versions, hooks, caches or external effects. Partial
online failure returns non-zero and keeps useful local configuration. Inspect the
reported failing command before retrying. Ctrl-C/timeout attempts to terminate the
child process tree; inspect active processes after forced OS termination.

Release SHA-256 files detect corruption, not a compromised publisher. Releases are
not Authenticode-signed or Apple Developer ID notarized. Never disable platform-wide
security checks to run them; verify the source/release and use the operating system's
per-application approval flow when needed.
