use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use url::Url;

pub const AGENTS: &str = include_str!("../AGENTS.md");
pub const APPEND: &str = include_str!("../APPEND_SYSTEM.md");
pub const PACKAGES: [(&str, &str, &str); 6] = [
    ("fff", "@ff-labs/pi-fff", "快速文件搜索"),
    ("lsp", "@narumitw/pi-lsp", "Language Server 集成"),
    ("usage", "pi-context-usage", "上下文用量显示"),
    ("prune", "pi-context-prune", "Agent 主动整理上下文"),
    ("web", "pi-web-search", "Web 搜索（服务可能另需认证）"),
    ("compact", "@lll9p/pi-better-compaction", "增强压缩与文本 fallback"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode { Enable, Disable, Keep }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model { pub provider: String, pub id: String, pub thinking: String }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Answers {
    pub schema: u32,
    pub packages: BTreeMap<String, Mode>,
    pub latest_packages: bool,
    pub rules: bool,
    pub code_mode: bool,
    pub tuning: bool,
    pub model: Option<Model>,
    pub summary_model: String,
    pub summary_thinking: String,
    pub steering_all: bool,
    pub network: String,
    pub public_proxy: String,
    pub private_proxy: String,
    // None preserves the existing provider proxy; Some("") removes it.
    pub provider_proxy: Option<String>,
    pub update_pi: bool,
    pub prefer_git_bash: bool,
}
impl Default for Answers {
    fn default() -> Self {
        Self {
            schema: 1, packages: PACKAGES.iter().map(|(id, _, _)| (id.to_string(), Mode::Enable)).collect(),
            latest_packages: false, rules: true, code_mode: true, tuning: true,
            model: None, summary_model: "openai/gpt-6-luna".into(), summary_thinking: "medium".into(),
            steering_all: false, network: "split".into(),
            public_proxy: "http://127.0.0.1:10809".into(), private_proxy: "http://127.0.0.1:10808".into(),
            provider_proxy: None, update_pi: true, prefer_git_bash: true,
        }
    }
}
impl Answers {
    pub fn mode(&self, id: &str) -> Mode { self.packages.get(id).copied().unwrap_or(Mode::Keep) }
    pub fn enabled(&self, id: &str) -> bool { self.mode(id) == Mode::Enable }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == 1, "Unsupported answers schema: {}", self.schema);
        for key in self.packages.keys() {
            ensure!(PACKAGES.iter().any(|(id, _, _)| id == key), "Unknown package ID: {key}");
        }
        ensure!(["split", "direct", "inherit"].contains(&self.network.as_str()), "network must be split/direct/inherit");
        proxy_url(&self.public_proxy)?; proxy_url(&self.private_proxy)?;
        if let Some(p) = &self.provider_proxy { if !p.is_empty() { proxy_url(p)?; } }
        if let Some(m) = &self.model {
            identifier(&m.provider)?; identifier(&m.id)?; thinking(&m.thinking)?;
        }
        identifier(&self.summary_model)?;
        ensure!(self.summary_model.contains('/'), "Summary model must be provider/model");
        thinking(&self.summary_thinking)?;
        Ok(())
    }
}
fn identifier(s: &str) -> Result<()> {
    ensure!(!s.is_empty() && s.len() <= 200 && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:".contains(c)), "Invalid model identifier (do not enter credentials)");
    Ok(())
}
pub fn thinking(s: &str) -> Result<()> {
    ensure!(["off", "minimal", "low", "medium", "high", "xhigh", "max"].contains(&s), "Invalid thinking level"); Ok(())
}
pub fn proxy_url(s: &str) -> Result<Url> {
    let u = Url::parse(s).context("Invalid proxy URL")?;
    ensure!(["http", "https"].contains(&u.scheme()) && u.host_str().is_some(), "Installer proxy must be http:// or https://; use the HTTP listener, not SOCKS, for npm");
    ensure!(u.username().is_empty() && u.password().is_none() && u.query().is_none() && u.fragment().is_none() && u.path() == "/", "Proxy URL must not contain credentials, query, fragment, or a path");
    Ok(u)
}
pub fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b { merge(a.entry(k.clone()).or_insert(Value::Null), v); }
        }
        (a, b) => *a = b.clone(),
    }
}
pub fn source(entry: &Value) -> Option<&str> { entry.as_str().or_else(|| entry.get("source").and_then(Value::as_str)) }
pub fn matches_package(entry: &Value, name: &str) -> bool {
    let base = format!("npm:{name}");
    source(entry).is_some_and(|s| s == base || s.strip_prefix(&base).is_some_and(|r| r.starts_with('@') && r.len() > 1))
}
pub fn packages(settings: &Value, a: &Answers) -> Result<Vec<Value>> {
    let mut out = match settings.get("packages") {
        None => Vec::new(), Some(v) => v.as_array().context("settings.packages must be an array")?.clone(),
    };
    for (id, name, _) in PACKAGES {
        let count = out.iter().filter(|e| matches_package(e, name)).count();
        ensure!(count <= 1, "Duplicate package declarations for {name}; reconcile them with pi config first");
        match a.mode(id) {
            Mode::Keep => {},
            Mode::Disable => out.retain(|e| !matches_package(e, name)),
            Mode::Enable => {
                if let Some(e) = out.iter_mut().find(|e| matches_package(e, name)) {
                    if a.latest_packages {
                        let s = Value::String(format!("npm:{name}"));
                        if e.is_object() { e["source"] = s; } else { *e = s; }
                    }
                } else { out.push(Value::String(format!("npm:{name}"))); }
            }
        }
    }
    Ok(out)
}
pub fn settings(current: &Value, a: &Answers, bash: Option<&str>) -> Result<Value> {
    ensure!(current.is_object(), "settings.json must contain an object");
    let mut out = current.clone();
    out["packages"] = Value::Array(packages(current, a)?);
    let mut tools = match out.get("defaultTools") {
        None => Vec::new(), Some(v) => v.as_array().context("defaultTools must be an array")?.clone(),
    };
    ensure!(tools.iter().all(Value::is_string), "defaultTools entries must be strings");
    tools.retain(|x| !["codemode", "+codemode", "-codemode"].contains(&x.as_str().unwrap_or("")));
    tools.push(json!(if a.code_mode { "+codemode" } else { "-codemode" }));
    out["defaultTools"] = json!(tools);
    // Code Mode has no 'off' mode: disable its tool instead.
    if a.code_mode { merge(&mut out, &json!({"codemode":{"mode":"only","inlineBudget":3000}})); }
    if a.tuning {
        merge(&mut out, &json!({"httpIdleTimeoutMs":300000,"transport":"sse",
            "retry":{"enabled":true,"maxRetries":3,"baseDelayMs":2000,"maxAgentDelayMs":60000,
                "provider":{"timeoutMs":300000,"maxRetries":0,"maxRetryDelayMs":60000}},
            "compaction":{"enabled":true,"reserveTokens":32768,"keepRecentTokens":20000}}));
    }
    if let Some(m) = &a.model {
        merge(&mut out, &json!({"defaultProvider":m.provider,"defaultModel":m.id,"defaultThinkingLevel":m.thinking}));
    }
    if let Some(proxy) = &a.provider_proxy {
        if proxy.is_empty() { out.as_object_mut().unwrap().remove("httpProxy"); }
        else { out["httpProxy"] = json!(proxy); }
    }
    if a.steering_all { out["steeringMode"] = json!("all"); }
    if a.prefer_git_bash { if let Some(b) = bash { out["shellPath"] = json!(b); } }
    // Never replace extensions/skills/prompts, change trust, or add credentials.
    Ok(out)
}
pub fn plugin_patches(a: &Answers) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    if a.enabled("fff") { out.push(("pi-fff.json", json!({"mode":"override","enableFsRootScanning":false,"enableHomeDirScanning":false,"warnOnHomeDirScan":true,"followSymlinks":true}))); }
    if a.enabled("lsp") { out.push(("lsp.json", json!({"timeout":30000,"servers":{
        "ty":{"command":["ty","server"],"extensions":[".py",".pyi"]},
        "ruff":{"command":["ruff","server"],"extensions":[".py",".pyi"]},
        "rust-analyzer":{"command":["rust-analyzer"],"extensions":[".rs"],"pullDiagnosticsGraceMs":5000}
    }}))); }
    for (id, file) in [("prune", "context-prune/settings.json"), ("compact", "extensions/pi-better-compaction/config.json")] {
        if a.mode(id) == Mode::Keep { continue; }
        let mut patch = json!({"enabled":a.enabled(id)});
        if a.enabled(id) {
            let more = if id == "prune" {
                json!({"summarizerModel":a.summary_model,"summarizerThinking":a.summary_thinking,
                    "pruneOn":"agentic-auto","batchingMode":"agent-message","remindUnprunedCount":true,
                    "showPruneStatusLine":true,"showStartupNotice":false,"notifySkipped":false})
            } else {
                json!({"compactionVersion":"v2","compactionModel":a.summary_model,"compactionThinkingLevel":a.summary_thinking,
                    "responsesCompactApis":["openai-responses","openai-codex-responses"],"allowCompactionContinuityBreak":false,
                    "notifyOnLoad":false,"debug":false,"logProviderPayloads":false,"logCompactResponses":false,"redactSensitiveData":true})
            };
            merge(&mut patch, &more);
        }
        out.push((file, patch));
    }
    out
}
fn clean_features(s: &str) -> String { s.lines().filter(|l| !l.starts_with("<!-- feature:")).collect::<Vec<_>>().join("\n") }
pub fn render_rules(a: &Answers) -> (String, String) {
    let agents = clean_features(AGENTS);
    let mut global = agents.split("## Network routing").next().unwrap_or(&agents).trim().to_owned();
    global.push_str("\n\n## Network routing\n\n");
    match a.network.as_str() {
        "split" => global.push_str(&format!(
            "Known domestic Chinese services and localhost/private/Tailscale endpoints: direct, unless an explicit endpoint rule says otherwise.\nPublic international downloads and anonymous GitHub access: {}.\nInternational requests carrying personal credentials/private data, including authenticated GitHub, Google API keys and OAuth CLIs: {}. Personal-authentication routing takes precedence.\nApply routing per process, overriding conflicting proxy and bypass variables. Never cycle routes, silently fall back to direct access, disable TLS verification, or change Pi's provider transport to fix a task download. A loopback proxy belongs to the executing host; do not assume it exists in SSH targets, containers, or remote tools. Diagnose the actual failure and continue independent authorized work. Never log credentials or claim a route was used without evidence.\n", a.public_proxy, a.private_proxy)),
        "direct" => global.push_str("Direct task networking is authorized on this machine. Clear conflicting inherited proxy settings for direct requests. Preserve explicit endpoint-specific routes. Do not change Pi's own provider transport. Do not expose credentials in diagnostics.\n"),
        _ => global.push_str("Use this machine's existing network configuration and explicit endpoint routing instructions. Do not invent proxy addresses or change routes blindly. Keep credentials private and scope failures to the affected operation.\n"),
    }
    let runtime = clean_features(APPEND);
    let mut supplement = runtime.split("## Code Mode").next().unwrap_or(&runtime).trim().to_owned();
    if a.code_mode {
        if let Some(section) = runtime.split("## Code Mode").nth(1) {
            supplement.push_str("\n\n## Code Mode");
            supplement.push_str(section.split("## Context extensions").next().unwrap_or(section).trim_end());
        }
    }
    if a.enabled("prune") || a.enabled("compact") {
        if let Some(section) = runtime.split("## Context extensions").nth(1) {
            supplement.push_str("\n\n## Context extensions"); supplement.push_str(section);
        }
    }
    (global, supplement)
}
pub fn checked_source(s: &str) -> Result<()> {
    let (_, name, _) = PACKAGES.iter().find(|(_, n, _)| matches_package(&json!(s), n)).context("Unknown package source")?;
    let base = format!("npm:{name}");
    let suffix = &s[base.len()..];
    if !suffix.is_empty() {
        let version = suffix.strip_prefix('@').context("Invalid npm source")?;
        ensure!(!version.is_empty() && version.len() <= 200 && version.chars().all(|c| c.is_ascii_alphanumeric() || "._-+^~*=!".contains(c)), "Unsupported npm version/tag; aliases, URLs, credentials and shell expressions are not accepted");
    }
    Ok(())
}
/// Pi update skips exact pins. Install missing/changed explicit versions instead.
pub fn package_action(s: &str, installed: Option<&str>) -> Result<Option<&'static str>> {
    checked_source(s)?;
    let (_, name, _) = PACKAGES.iter().find(|(_, n, _)| matches_package(&json!(s), n)).context("Unknown package source")?;
    let Some(installed) = installed else { return Ok(Some("install")); };
    let suffix = &s[format!("npm:{name}").len()..];
    if let Some(wanted) = suffix.strip_prefix('@') {
        if wanted.trim_start_matches('v') == installed { return Ok(None); }
        return Ok(Some("install"));
    }
    Ok(Some("update"))
}
