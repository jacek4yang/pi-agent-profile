use crate::{config::{self, Answers}, system};
use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fs::{self, File, OpenOptions}, io::Write, path::{Component, Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

pub const STATE: &str = ".pi-profile/state.json";
const BEGIN: &str = "<!-- pi-profile:begin -->";
const END: &str = "<!-- pi-profile:end -->";
pub const TARGETS: [&str; 8] = ["AGENTS.md", "APPEND_SYSTEM.md", "settings.json", "pi-fff.json", "lsp.json", "context-prune/settings.json", "extensions/pi-better-compaction/config.json", STATE];
const LIMIT: u64 = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saved { pub schema: u32, pub version: String, pub answers: Answers, pub blocks: BTreeMap<String, String> }
#[derive(Clone, Debug)]
pub struct Change { pub path: String, pub before: Option<Vec<u8>>, pub after: Option<Vec<u8>> }
#[derive(Debug)]
pub struct Plan { pub changes: Vec<Change>, pub sources: Vec<String> }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry { path: String, before: Option<String>, after: Option<String> }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest { schema: u32, agent_dir: PathBuf, files: Vec<Entry> }

pub fn digest(b: &[u8]) -> String { format!("{:x}", Sha256::digest(b)) }
fn optional_hash(b: &Option<Vec<u8>>) -> Option<String> { b.as_ref().map(|b| digest(b)) }
pub fn json_bytes<T: Serialize>(v: &T) -> Result<Vec<u8>> { let mut b = serde_json::to_vec_pretty(v)?; b.push(b'\n'); Ok(b) }
pub fn safe_path(root: &Path, rel: &str) -> Result<PathBuf> {
    ensure!(!rel.is_empty() && !rel.contains('\\'), "Invalid relative path");
    ensure!(Path::new(rel).components().all(|c| matches!(c, Component::Normal(_))), "Unsafe relative path: {rel}");
    let mut p = root.to_path_buf();
    if let Ok(m) = fs::symlink_metadata(&p) { ensure!(!m.file_type().is_symlink() && m.is_dir(), "Agent root must be a real directory"); }
    for part in Path::new(rel).components() {
        p.push(part.as_os_str());
        match fs::symlink_metadata(&p) {
            Ok(m) => ensure!(!m.file_type().is_symlink(), "Refusing symlink: {}", p.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(e).with_context(|| format!("Inspect {}", p.display())),
        }
    }
    Ok(p)
}
pub fn read_optional(root: &Path, rel: &str) -> Result<Option<Vec<u8>>> {
    let p = safe_path(root, rel)?;
    match fs::metadata(&p) {
        Ok(m) => { ensure!(m.is_file() && m.len() <= LIMIT, "Not a regular small file: {}", p.display()); Ok(Some(fs::read(p)?)) },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn read_json(root: &Path, rel: &str) -> Result<Value> {
    let v = match read_optional(root, rel)? {
        Some(b) => serde_json::from_slice(&b).with_context(|| format!("Invalid JSON in {rel}; original left unchanged"))?,
        None => json!({}),
    };
    ensure!(v.is_object(), "{rel} must contain a JSON object"); Ok(v)
}
pub fn load(root: &Path) -> Result<Option<Saved>> {
    let Some(b) = read_optional(root, STATE)? else { return Ok(None) };
    let saved: Saved = serde_json::from_slice(&b).context("Invalid profile state; do not overwrite it blindly")?;
    ensure!(saved.schema == 1, "Unsupported profile state schema"); saved.answers.validate()?; Ok(Some(saved))
}
fn block(s: &str) -> Result<Option<&str>> {
    match (s.matches(BEGIN).count(), s.matches(END).count()) {
        (0, 0) => Ok(None),
        (1, 1) => { let start = s.find(BEGIN).unwrap(); let end = s.find(END).unwrap(); ensure!(end > start, "Invalid rule marker ordering"); Ok(Some(&s[start..end + END.len()])) },
        _ => bail!("Duplicate or incomplete pi-profile markers; repair the rule file first"),
    }
}
fn normalized(s: &str) -> String { s.lines().filter(|l| !l.starts_with("<!-- feature:")).collect::<Vec<_>>().join("\n").trim().to_owned() }
pub fn replace_block(existing: &str, body: Option<&str>, expected: Option<&str>, overwrite: bool, legacy: &str) -> Result<String> {
    let old = block(existing)?;
    if let Some(old) = old {
        if let Some(hash) = expected { ensure!(digest(old.as_bytes()) == hash || overwrite, "Managed rule block was edited; inspect it or use --overwrite-managed"); }
        else { ensure!(overwrite, "Managed block has no ownership state; inspect it before --overwrite-managed"); }
    } else if expected.is_some() { ensure!(overwrite, "Managed rule block was removed; inspect it before --overwrite-managed"); }
    let new = body.map(|b| format!("{BEGIN}\n{}\n{END}", b.trim())).unwrap_or_default();
    if let Some(old) = old { return Ok(existing.replacen(old, &new, 1)); }
    if new.is_empty() { return Ok(existing.to_owned()); }
    if existing.trim().is_empty() || normalized(existing) == normalized(legacy) { return Ok(format!("{new}\n")); }
    Ok(format!("{}\n\n{new}\n", existing.trim_end()))
}
pub fn plan(root: &Path, a: &Answers, overwrite: bool) -> Result<Plan> {
    a.validate()?;
    let previous = load(root)?;
    if a.rules { ensure!(read_optional(root, "AGENTS.override.md")?.is_none(), "AGENTS.override.md shadows global rules; resolve it first or decline managed rules"); }
    let current = read_json(root, "settings.json")?;
    let bash = if cfg!(windows) && a.prefer_git_bash { system::git_bash().map(|p| p.to_string_lossy().into_owned()) } else { None };
    let settings = config::settings(&current, a, bash.as_deref())?;
    let mut sources = Vec::new();
    for (id, name, _) in config::PACKAGES {
        if a.enabled(id) {
            let entry = settings["packages"].as_array().unwrap().iter().find(|e| config::matches_package(e, name)).unwrap();
            let s = config::source(entry).context("Package entry has no source")?;
            config::checked_source(s)?; sources.push(s.to_owned());
        }
    }
    let mut desired = BTreeMap::new();
    desired.insert("settings.json".to_owned(), json_bytes(&settings)?);
    for (file, patch) in config::plugin_patches(a) {
        let mut v = read_json(root, file)?; config::merge(&mut v, &patch); desired.insert(file.into(), json_bytes(&v)?);
    }
    let (global, runtime) = config::render_rules(a);
    let mut hashes = BTreeMap::new();
    for (file, body, legacy) in [("AGENTS.md", global, config::AGENTS), ("APPEND_SYSTEM.md", runtime, config::APPEND)] {
        let old = read_optional(root, file)?.unwrap_or_default();
        let old = std::str::from_utf8(&old).with_context(|| format!("{file} is not UTF-8"))?;
        let expected = previous.as_ref().and_then(|s| s.blocks.get(file)).map(String::as_str);
        let result = replace_block(old, a.rules.then_some(body.as_str()), expected, overwrite, legacy)?;
        if let Some(b) = block(&result)? { hashes.insert(file.to_owned(), digest(b.as_bytes())); }
        if !result.is_empty() || !old.is_empty() { desired.insert(file.into(), result.into_bytes()); }
    }
    let saved = Saved { schema: 1, version: env!("CARGO_PKG_VERSION").into(), answers: a.clone(), blocks: hashes };
    desired.insert(STATE.into(), json_bytes(&saved)?);
    let mut changes = Vec::new();
    for (file, after) in desired {
        let before = read_optional(root, &file)?;
        let equivalent = file.ends_with(".json") && before.as_ref().is_some_and(|b| {
            serde_json::from_slice::<Value>(b).ok() == serde_json::from_slice::<Value>(&after).ok()
        });
        if !equivalent && before.as_deref() != Some(after.as_slice()) { changes.push(Change { path: file, before, after: Some(after) }); }
    }
    Ok(Plan { changes, sources })
}
pub fn private_dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p)?;
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(p, fs::Permissions::from_mode(0o700))?; }
    Ok(())
}
fn atomic(p: &Path, bytes: &[u8]) -> Result<()> {
    let parent = p.parent().context("No parent directory")?;
    fs::create_dir_all(parent)?;
    let mut t = tempfile::NamedTempFile::new_in(parent)?;
    t.write_all(bytes)?; t.as_file().sync_all()?;
    // Sensitive existing settings are never made more readable.
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(p).map(|m| m.permissions().mode() & 0o600).unwrap_or(0o600);
        t.as_file().set_permissions(fs::Permissions::from_mode(mode | 0o600))?;
    }
    t.persist(p).map_err(|e| e.error)?;
    #[cfg(unix)] File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn lock(root: &Path) -> Result<File> {
    let meta = safe_path(root, ".pi-profile")?; private_dir(&meta)?;
    let lock = safe_path(root, ".pi-profile/lock")?;
    let f = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(lock)?;
    f.try_lock_exclusive().context("Another pi-profile process is changing this environment")?; Ok(f)
}
pub fn pending(root: &Path) -> Result<Option<String>> {
    match read_optional(root, ".pi-profile/pending.json")? {
        None => Ok(None),
        Some(b) => { let id: String = serde_json::from_slice(&b).context("Invalid pending journal")?; valid_id(&id)?; Ok(Some(id)) },
    }
}
fn valid_id(id: &str) -> Result<()> { ensure!(!id.is_empty() && id.bytes().all(|c| c.is_ascii_digit() || c == b'-'), "Invalid backup ID"); Ok(()) }
pub fn backups(root: &Path) -> Result<Vec<String>> {
    let p = safe_path(root, ".pi-profile/backups")?;
    if !p.exists() { return Ok(Vec::new()); }
    let mut ids = Vec::new();
    for e in fs::read_dir(p)? {
        let e = e?; let id = e.file_name().to_string_lossy().into_owned();
        if e.file_type()?.is_dir() && valid_id(&id).is_ok() { ids.push(id); }
    }
    ids.sort(); ids.reverse(); Ok(ids)
}
pub fn commit(root: &Path, changes: &[Change], recovery: bool) -> Result<Option<String>> {
    if changes.is_empty() { return Ok(None); }
    ensure!(recovery || pending(root)?.is_none(), "Incomplete previous transaction; run restore first");
    let mut names = BTreeSet::new();
    for c in changes {
        ensure!(TARGETS.contains(&c.path.as_str()) && names.insert(&c.path), "Invalid/duplicate managed target");
        ensure!(read_optional(root, &c.path)? == c.before, "{} changed after planning; rerun instead of overwriting concurrent work", c.path);
    }
    let id = format!("{}-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(), std::process::id());
    let backup = safe_path(root, &format!(".pi-profile/backups/{id}"))?;
    private_dir(&backup)?;
    let mut entries = Vec::new();
    for c in changes {
        if let Some(b) = &c.before { atomic(&safe_path(&backup, &format!("files/{}", c.path))?, b)?; }
        entries.push(Entry { path: c.path.clone(), before: optional_hash(&c.before), after: optional_hash(&c.after) });
    }
    let m = Manifest { schema: 1, agent_dir: fs::canonicalize(root)?, files: entries };
    atomic(&backup.join("manifest.json"), &json_bytes(&m)?)?;
    let journal = safe_path(root, ".pi-profile/pending.json")?;
    atomic(&journal, &json_bytes(&id)?)?;
    for c in changes {
        ensure!(read_optional(root, &c.path)? == c.before, "{} changed during apply; recover backup {id}", c.path);
        let p = safe_path(root, &c.path)?;
        match &c.after { Some(b) => atomic(&p, b)?, None if p.exists() => fs::remove_file(p)?, None => {} }
    }
    fs::remove_file(journal)?;
    println!("Backup: {id}"); Ok(Some(id))
}
pub fn restore_plan(root: &Path, id: &str, force: bool) -> Result<Vec<Change>> {
    valid_id(id)?;
    let dir = safe_path(root, &format!(".pi-profile/backups/{id}"))?;
    let raw = read_optional(&dir, "manifest.json")?.context("Backup manifest is missing")?;
    let m: Manifest = serde_json::from_slice(&raw).context("Invalid backup manifest")?;
    ensure!(m.schema == 1 && m.agent_dir == fs::canonicalize(root)?, "Backup belongs to another agent directory/schema");
    let mut names = BTreeSet::new(); let mut changes = Vec::new();
    for entry in m.files {
        ensure!(TARGETS.contains(&entry.path.as_str()) && names.insert(entry.path.clone()), "Unsafe/duplicate backup target");
        let before = read_optional(root, &entry.path)?;
        let hash = optional_hash(&before);
        ensure!(force || hash == entry.after || hash == entry.before, "{} changed since this backup; inspect it before --force", entry.path);
        let after = if entry.before.is_some() { read_optional(&dir, &format!("files/{}", entry.path))? } else { None };
        ensure!(optional_hash(&after) == entry.before, "Backup integrity failure: {}", entry.path);
        if before != after { changes.push(Change { path: entry.path, before, after }); }
    }
    Ok(changes)
}
pub fn finish_empty_restore(root: &Path, id: &str) -> Result<()> {
    if pending(root)?.as_deref() == Some(id) {
        ensure!(restore_plan(root, id, false)?.is_empty(), "Restore is not complete");
        fs::remove_file(safe_path(root, ".pi-profile/pending.json")?)?;
    }
    Ok(())
}
