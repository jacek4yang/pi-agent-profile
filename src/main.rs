use anyhow::{ensure, Context, Result};
use clap::{Parser, Subcommand};
use pi_profile::{config::{self, Answers, Mode, Model, PACKAGES}, storage, system};
use serde_json::Value;
use std::{env, fs, io::{self, IsTerminal, Write}, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(version, about = "Pi Agent profile: interactive setup, selective updates and recoverable configuration")]
struct Cli {
    #[command(subcommand)] command: Option<Action>,
    #[arg(long, global = true)] agent_dir: Option<PathBuf>,
    #[arg(long, global = true)] answers: Option<PathBuf>,
    #[arg(long, global = true)] yes: bool,
    #[arg(long, global = true)] offline: bool,
    #[arg(long, global = true)] dry_run: bool,
    #[arg(long, global = true)] overwrite_managed: bool,
    #[arg(long, global = true)] no_self_update: bool,
    #[arg(long, global = true, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..=3600))] timeout: u64,
}
#[derive(Subcommand, Clone)]
enum Action {
    /// Interactive questions, followed by an explicit apply confirmation.
    Configure,
    /// Reapply saved selections and update only selected Pi packages.
    Update,
    /// Show changed filenames without writing files or using the network.
    Plan,
    /// Inspect configuration, installed package files and local prerequisites.
    Doctor { #[arg(long)] strict: bool },
    /// List local recovery snapshots.
    Backups,
    /// Restore files, not Pi/plugin executable versions.
    Restore { #[arg(long)] backup: Option<String>, #[arg(long)] force: bool },
    /// Export non-secret selections; never copies Pi credentials or sessions.
    Export { #[arg(long)] output: PathBuf },
}
fn ask(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: "); io::stdout().flush()?;
    let mut line = String::new();
    ensure!(io::stdin().read_line(&mut line)? > 0, "Input closed; no implicit approval");
    let line = line.trim();
    Ok(if line.is_empty() { default.to_owned() } else { line.to_owned() })
}
fn yn(label: &str, default: bool) -> Result<bool> {
    loop {
        match ask(label, if default { "Y/n" } else { "y/N" })?.to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true), "n" | "no" => return Ok(false),
            "y/n" => return Ok(default), _ => println!("请输入 y 或 n。"),
        }
    }
}
fn terminal() -> Result<()> { ensure!(io::stdin().is_terminal() && io::stdout().is_terminal(), "Interactive setup needs a terminal. For automation use configure --answers FILE --yes [--offline]"); Ok(()) }
fn wizard(mut a: Answers, existing: &Value) -> Result<Answers> {
    terminal()?;
    println!("\nPi Profile 配置向导。不会询问、读取或保存 API key / OAuth token。\n回车采用括号中的默认值；n 表示不启用该选项。\n");
    a.rules = yn("管理 AGENTS.md 和 APPEND_SYSTEM.md（保留个人内容）?", a.rules)?;
    a.code_mode = yn("启用 Code Mode only?", a.code_mode)?;
    for (id, name, help) in PACKAGES {
        let enabled = yn(&format!("启用 {name} — {help}?"), a.enabled(id))?;
        let present = existing["packages"].as_array().is_some_and(|p| p.iter().any(|e| config::matches_package(e, name)));
        let mode = if enabled { Mode::Enable } else if present && !yn("该插件已在全局配置中，解除其全局加载？（不删除缓存）", false)? { Mode::Keep } else { Mode::Disable };
        a.packages.insert(id.to_owned(), mode);
    }
    a.latest_packages = yn("取消已选插件的版本锁定，跟随最新版本？（n 保留已有 pin）", a.latest_packages)?;
    a.tuning = yn("应用推荐超时/重试/基础压缩预算？（n 保留现值）", a.tuning)?;
    if yn("指定默认 provider / model？（n 不更改当前默认模型）", a.model.is_some())? {
        let old = a.model.clone();
        a.model = Some(Model {
            provider: ask("Provider", old.as_ref().map(|m| m.provider.as_str()).or_else(|| existing["defaultProvider"].as_str()).unwrap_or("openai"))?,
            id: ask("Model ID（以本机 pi --list-models 为准）", old.as_ref().map(|m| m.id.as_str()).or_else(|| existing["defaultModel"].as_str()).unwrap_or("gpt-6-astra"))?,
            thinking: ask("Thinking: off/minimal/low/medium/high/xhigh/max", old.as_ref().map(|m| m.thinking.as_str()).unwrap_or("medium"))?,
        });
    } else { a.model = None; }
    if a.enabled("prune") || a.enabled("compact") {
        println!("摘要模型必须在本机可用；这些插件会额外调用模型，安装器不执行模型请求。 ");
        a.summary_model = ask("摘要模型 provider/model", &a.summary_model)?;
        a.summary_thinking = ask("摘要 thinking", &a.summary_thinking)?;
    }
    if yn("修改高级选项（steering / Windows shell）?", false)? {
        a.steering_all = yn("设置 steeringMode=all？（实验性；n 保留现值）", a.steering_all)?;
        a.prefer_git_bash = yn("Windows 自动优先使用 Git Bash?", a.prefer_git_bash)?;
    }
    println!("网络模式：split=公开/个人认证分流；direct=明确直连；inherit=继承进程环境。 ");
    a.network = ask("网络模式", &a.network)?;
    if a.network == "split" {
        a.public_proxy = ask("国外公开下载 HTTP proxy", &a.public_proxy)?;
        a.private_proxy = ask("个人认证请求 HTTP proxy（写入 Agent 规则）", &a.private_proxy)?;
    }
    if yn("修改 Pi 模型请求的代理？（独立于上述下载代理）", a.provider_proxy.is_some())? {
        let value = ask("输入代理 URL；direct 表示清除 httpProxy", a.provider_proxy.as_deref().filter(|p| !p.is_empty()).unwrap_or("http://127.0.0.1:10810"))?;
        a.provider_proxy = Some(if value == "direct" { String::new() } else { value });
    } else { a.provider_proxy = None; }
    a.update_pi = yn("本次及后续更新允许安装/更新 Pi 核心?", a.update_pi)?;
    a.validate()?; Ok(a)
}
fn resolve_dir(cli: &Cli) -> Result<PathBuf> {
    let home = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).context("Home directory is unavailable; use --agent-dir")?;
    let p = cli.agent_dir.clone().or_else(|| env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from)).unwrap_or_else(|| PathBuf::from(&home).join(".pi/agent"));
    let p = if let Ok(tail) = p.strip_prefix("~") { PathBuf::from(home).join(tail) } else { p };
    let p = std::path::absolute(p)?;
    ensure!(p.parent().is_some(), "Refusing filesystem root as agent directory");
    storage::safe_path(&p, "settings.json")?;
    if p.exists() { return Ok(fs::canonicalize(p)?); }
    Ok(p)
}
fn show_plan(plan: &storage::Plan, a: &Answers) {
    println!("\nPlan: {} changed files; unknown settings/packages and credentials are preserved.", plan.changes.len());
    for c in &plan.changes { println!("  {} {}", if c.before.is_some() { "update" } else { "create" }, c.path); }
    for (id, name, _) in PACKAGES { println!("  {:?}: {name}", a.mode(id)); }
    println!("Network: {}; public route: {}", a.network, if a.network == "split" { &a.public_proxy } else { &a.network });
    println!("Configuration snapshots do not roll back installed Pi/plugin binaries. Close other Pi processes before applying.");
}
fn confirm(cli: &Cli, label: &str) -> Result<bool> {
    if cli.yes { return Ok(true); } terminal()?; yn(label, false)
}
fn main() {
    if let Err(e) = run(Cli::parse()) { eprintln!("ERROR: {e:#}"); std::process::exit(1); }
}
fn run(cli: Cli) -> Result<()> {
    let root = resolve_dir(&cli)?;
    let saved = if matches!(&cli.command, Some(Action::Restore { .. } | Action::Backups | Action::Doctor { .. })) { None } else { storage::load(&root)? };
    let action = if let Some(action) = &cli.command { action.clone() } else {
        terminal()?;
        if saved.is_none() { Action::Configure } else {
            println!("Pi Profile {} — {}\n1 修改功能/配置\n2 更新已选择的 Pi/插件\n3 查看配置差异\n4 诊断\n5 备份/恢复\n6 退出", env!("CARGO_PKG_VERSION"), root.display());
            match ask("选择", "1")?.as_str() { "1" => Action::Configure, "2" => Action::Update, "3" => Action::Plan, "4" => Action::Doctor { strict: false }, "5" => Action::Restore { backup: None, force: false }, "6" => return Ok(()), _ => anyhow::bail!("Invalid menu choice") }
        }
    };
    match action {
        Action::Doctor { strict } => { let failures = system::doctor(&root)?; ensure!(!strict || failures == 0, "Doctor found {failures} unresolved profile checks"); return Ok(()); },
        Action::Backups => { for id in storage::backups(&root)? { println!("{id}"); } return Ok(()); },
        Action::Restore { backup, force } => {
            let id = match backup.or(storage::pending(&root)?) {
                Some(id) => id,
                None => { let ids = storage::backups(&root)?; ensure!(!ids.is_empty(), "No backups"); println!("Backups (newest first):\n{}", ids.join("\n")); if cli.yes { ids[0].clone() } else { terminal()?; ask("恢复的 backup ID", &ids[0])? } },
            };
            let changes = storage::restore_plan(&root, &id, force)?;
            println!("Restore {id}: {} files; Pi/plugin versions will not be downgraded", changes.len());
            if cli.dry_run || !confirm(&cli, "确认恢复？恢复前也会备份当前文件")? { return Ok(()); }
            let _lock = storage::lock(&root)?;
            storage::commit(&root, &changes, true)?; storage::finish_empty_restore(&root, &id)?;
            println!("Restored. Restart Pi to reload all resources."); return Ok(());
        },
        Action::Export { output } => {
            let a = saved.context("No saved selections to export")?.answers;
            if cli.dry_run { println!("Would export non-secret choices to {}", output.display()); return Ok(()); }
            let mut options = fs::OpenOptions::new(); options.write(true).create_new(true);
            #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
            let mut file = options.open(&output).context("Export destination must not already exist")?;
            file.write_all(&storage::json_bytes(&a)?)?; file.sync_all()?;
            println!("Exported selections only: {}", output.display()); return Ok(());
        },
        _ => {},
    }
    ensure!(storage::pending(&root)?.is_none(), "Interrupted transaction found; run restore before another apply");
    let mut a = if let Some(p) = &cli.answers {
        ensure!(fs::metadata(p)?.len() < 1024 * 1024, "Answers file is too large");
        serde_json::from_slice::<Answers>(&fs::read(p)?).context("Invalid answers file")?
    } else { saved.map(|s| s.answers).unwrap_or_default() };
    if matches!(action, Action::Configure) && cli.answers.is_none() {
        ensure!(!cli.yes, "configure --yes requires an explicit --answers file");
        a = wizard(a, &storage::read_json(&root, "settings.json")?)?;
    } else if matches!(action, Action::Update) && storage::load(&root)?.is_none() && cli.answers.is_none() {
        anyhow::bail!("No saved choices; run configure first");
    }
    let plan = storage::plan(&root, &a, cli.overwrite_managed)?;
    show_plan(&plan, &a);
    if cli.dry_run || matches!(action, Action::Plan) { return Ok(()); }
    if !confirm(&cli, "确认写入配置并执行选定的更新（--offline 则只写配置）?")? { println!("Cancelled; no changes made."); return Ok(()); }
    let _lock = storage::lock(&root)?;
    storage::commit(&root, &plan.changes, false)?;
    if cli.offline { println!("Local configuration saved. Offline mode: package installation/update NOT performed."); }
    else { if cli.no_self_update { a.update_pi = false; } system::sync(&root, &a, &plan.sources, Duration::from_secs(cli.timeout))?; }
    println!("Restart Pi after applying changes. Configuration success does not verify model availability or live extension behavior.");
    Ok(())
}
