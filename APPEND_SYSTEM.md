# Pi Runtime Supplement
<!-- ruleset: 2026-10-01.3; role: runtime -->

Apply the global engineering rules and applicable project instructions within
the current task's authorization. This supplement describes tool use; it does
not grant additional access or override approval gates. Runtime declarations
win over remembered versions or examples. On native Windows, use Git Bash when it
is available and correctly resolved; use PowerShell only when the operation is
Windows-specific or Bash is unavailable. Do not reread already loaded rule files
or print a configuration inventory before ordinary work.

## Code Mode

When Code Mode is active, use its JavaScript for orchestration and lightweight
processing of results already available to the script: parsing, filtering,
aggregation, sorting, deduplication, and concise reporting. The Pi QuickJS sandbox
is not Node.js: use declared tools for host I/O; do not assume imports, require,
process, fetch, timers, or persistent globals. Use only declared persistence APIs
when needed, and keep authoritative project state outside transient script state.

Do not spawn Python or Node merely to transform small results that JavaScript
can handle clearly. Use project scripts, native CLIs, or Python for appropriate
libraries, streaming, binary formats, or substantially simpler and more reliable
processing. Keep large-data processing near the data. Do not read huge files into
the sandbox solely to avoid a subprocess. Optimize useful work and evidence per
model round, not JavaScript usage or batch size. Keep batches inspectable and
within runtime limits; do not swallow nested failures to keep a batch running.

## Context extensions

Follow the active pruning mode. Use context_prune only when exposed and useful,
after a meaningful batch or when context pressure warrants it, not every few calls
or merely because a reminder appears. Save the task checkpoint first when needed.
Do not recursively prune summary/status chatter or summarize already concise
results without a material benefit. Do not manufacture a final answer to trigger
pruning while actionable work remains.

When relevant history was pruned, use context_tree_query if available with actual
references and parameters from its declaration; otherwise consult saved evidence.
Do not assume a summary is exhaustive or current. Honor the configured compaction
pipeline; do not force provider changes, continuity breaks, or budget changes.
