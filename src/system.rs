use crate::{config::{self, Answers, PACKAGES}, storage};
use anyhow::{bail, ensure, Context, Result};
use std::{env, fs, io::{Read, Seek, SeekFrom}, net::{TcpStream, ToSocketAddrs}, path::{Path, PathBuf}, process::{Command, Stdio}, thread, time::{Duration, Instant}};

pub fn find_program(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").or_else(|| env::var_os("Path"))?;
    let suffixes: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat", ""] } else { &[""] };
    for dir in env::split_paths(&path) {
        if dir.as_os_str().is_empty() { continue; }
        for ext in suffixes {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; if fs::metadata(&p).ok()?.permissions().mode() & 0o111 == 0 { continue; } }
                return Some(p);
            }
        }
    }
    None
}
pub fn git_bash() -> Option<PathBuf> {
    if !cfg!(windows) { return None; }
    let mut candidates = Vec::new();
    for key in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = env::var_os(key) { candidates.push(PathBuf::from(p).join("Git/bin/bash.exe")); }
    }
    if let Some(p) = env::var_os("LOCALAPPDATA") { candidates.push(PathBuf::from(p).join("Programs/Git/bin/bash.exe")); }
    if let Some(git) = find_program("git") {
        for parent in git.ancestors().skip(1).take(3) { candidates.push(parent.join("bin/bash.exe")); candidates.push(parent.join("usr/bin/bash.exe")); }
    }
    if let Some(p) = find_program("bash") {
        // Never pick the WSL launcher accidentally.
        if !p.to_string_lossy().to_ascii_lowercase().contains("windows\\system32") { candidates.push(p); }
    }
    candidates.into_iter().find(|p| p.is_file())
}
pub fn command_for(program: &str, args: &[&str]) -> Result<Command> {
    let executable = find_program(program).with_context(|| format!("{program} is not on PATH"))?;
    #[cfg(windows)] {
        if let Some(bash) = git_bash() {
            let executable = if [Some("cmd"), Some("bat")].contains(&executable.extension().and_then(|x| x.to_str())) && executable.with_extension("").is_file() { executable.with_extension("") } else { executable.clone() };
            if executable.extension().and_then(|x| x.to_str()) != Some("cmd") && executable.extension().and_then(|x| x.to_str()) != Some("bat") {
                let mut c = Command::new(bash);
                c.args(["--noprofile", "--norc", "-c", "exec \"$@\"", "pi-profile"]);
                c.arg(executable.to_string_lossy().replace('\\', "/")).args(args);
                c.env("MSYS_NO_PATHCONV", "1").env("MSYS2_ARG_CONV_EXCL", "*"); return Ok(c);
            }
        }
        if [Some("cmd"), Some("bat")].contains(&executable.extension().and_then(|x| x.to_str())) {
            let base = executable.parent().context("Shim has no parent")?;
            let scripts: &[&str] = match program {
                "npm" => &["node_modules/npm/bin/npm-cli.js"],
                "pi" => &["node_modules/@earendil-works/pi-coding-agent/dist/cli.js", "node_modules/@mariozechner/pi-coding-agent/dist/cli.js"],
                _ => &[],
            };
            let script = scripts.iter().map(|s| base.join(s)).find(|p| p.is_file()).context("Cannot safely resolve this Windows shim; install Git Bash or repair the npm installation")?;
            let node = find_program("node").context("Node.js is missing")?;
            let mut c = Command::new(node); c.arg(script).args(args); return Ok(c);
        }
    }
    let mut c = Command::new(executable); c.args(args); Ok(c)
}
pub fn proxy_available(proxy: &str) -> Result<bool> {
    let u = config::proxy_url(proxy)?;
    let host = u.host_str().context("No proxy host")?;
    let port = u.port_or_known_default().context("No proxy port")?;
    let addrs = (host, port).to_socket_addrs()?;
    Ok(addrs.take(4).any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(700)).is_ok()))
}
pub fn configure_env(c: &mut Command, root: &Path, a: &Answers, clean: &Path, offline: bool) {
    // Isolate public package operations from npmrc credentials and preload hooks.
    for (key, _) in env::vars_os() {
        let k = key.to_string_lossy().to_ascii_lowercase();
        if k.starts_with("npm_config_") || k.ends_with("_token") || k.ends_with("_api_key") || ["node_options", "bun_options", "bash_env", "env"].contains(&k.as_str()) { c.env_remove(key); }
    }
    c.current_dir(clean).env("PI_CODING_AGENT_DIR", root).env("PI_TELEMETRY", "0").env("PI_SKIP_VERSION_CHECK", "1");
    if offline { c.env("PI_OFFLINE", "1"); } else { c.env_remove("PI_OFFLINE"); }
    c.env("npm_config_userconfig", clean.join("user.npmrc"))
        .env("npm_config_globalconfig", clean.join("global.npmrc"))
        .env("npm_config_registry", "https://registry.npmjs.org/")
        .env("npm_config_audit", "false").env("npm_config_fund", "false")
        .env("npm_config_fetch_retries", "1").env("npm_config_fetch_timeout", "60000");
    if a.network != "inherit" || offline {
        // Empty-but-present HTTP_PROXY prevents Pi's httpProxy ??= default from taking over.
        let route = if a.network == "split" && !offline { a.public_proxy.as_str() } else { "" };
        for k in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy", "npm_config_proxy", "npm_config_https_proxy"] { c.env(k, route); }
        let bypass = if route.is_empty() { "*" } else { "localhost,127.0.0.1,::1" };
        for k in ["NO_PROXY", "no_proxy", "npm_config_noproxy"] { c.env(k, bypass); }
    }
}
fn clean_dir() -> Result<tempfile::TempDir> {
    let t = tempfile::tempdir()?;
    fs::write(t.path().join("user.npmrc"), "")?; fs::write(t.path().join("global.npmrc"), "")?;
    Ok(t)
}
fn terminate(child: &mut std::process::Child) {
    #[cfg(unix)] { let _ = Command::new("/bin/kill").args(["-KILL", "--", &format!("-{}", child.id())]).status(); }
    #[cfg(windows)] { let _ = Command::new("taskkill.exe").args(["/PID", &child.id().to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    let _ = child.kill(); let _ = child.wait();
}
static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static HANDLER: std::sync::OnceLock<std::result::Result<(), String>> = std::sync::OnceLock::new();
pub fn execute(mut c: Command, timeout: Duration, capture: bool) -> Result<String> {
    let handler = HANDLER.get_or_init(|| ctrlc::set_handler(|| {
        CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
    }).map_err(|e| e.to_string()));
    ensure!(handler.is_ok(), "Cannot install cancellation handler");
    ensure!(!CANCELLED.load(std::sync::atomic::Ordering::SeqCst), "Cancelled");
    c.stdin(Stdio::null());
    let mut output = tempfile::tempfile()?;
    if capture { c.stdout(Stdio::from(output.try_clone()?)).stderr(Stdio::null()); }
    else { c.stdout(Stdio::inherit()).stderr(Stdio::inherit()); }
    #[cfg(unix)] { use std::os::unix::process::CommandExt; c.process_group(0); }
    let mut child = c.spawn().context("Could not start child process")?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? { ensure!(status.success(), "Command failed with {status}"); break; }
        if CANCELLED.load(std::sync::atomic::Ordering::SeqCst) { terminate(&mut child); bail!("Cancelled; child-process termination attempted"); }
        if started.elapsed() >= timeout { terminate(&mut child); bail!("Command timed out; process-tree termination attempted. Inspect state before retrying"); }
        thread::sleep(Duration::from_millis(100));
    }
    output.seek(SeekFrom::Start(0))?;
    let mut b = Vec::new(); output.take(65536).read_to_end(&mut b)?;
    Ok(String::from_utf8_lossy(&b).trim().to_owned())
}
pub fn version(name: &str, root: &Path) -> Result<String> {
    let mut c = command_for(name, &["--version"])?; let clean = clean_dir()?;
    configure_env(&mut c, root, &Answers::default(), clean.path(), true);
    execute(c, Duration::from_secs(15), true)
}
pub fn node_supported(v: &str) -> bool {
    let parts: Vec<_> = v.trim().trim_start_matches('v').split('.').filter_map(|s| s.parse::<u64>().ok()).collect();
    parts.len() == 3 && (parts[0], parts[1], parts[2]) >= (22, 19, 0)
}
pub fn sync(root: &Path, a: &Answers, sources: &[String], timeout: Duration) -> Result<()> {
    if sources.is_empty() && !a.update_pi { println!("No online operations selected."); return Ok(()); }
    if a.network == "split" {
        ensure!(proxy_available(&a.public_proxy)?, "Required public proxy is unavailable; no alternate route attempted. Local configuration is already saved; rerun update later");
    }
    let clean = clean_dir()?;
    let node = version("node", root).context("Node.js >=22.19 is required for npm-based Pi/packages; the Rust configurator itself does not require Node")?;
    ensure!(node_supported(&node), "Node.js >=22.19 required; found {node}");
    let settings = storage::read_json(root, "settings.json")?;
    if let Some(cmd) = settings.get("npmCommand") {
        ensure!(cmd == &serde_json::json!(["npm"]), "Custom npmCommand found; it may override routing. Keep it and run package management manually, or explicitly migrate it first");
    }
    let run = |program: &str, args: &[&str]| -> Result<()> {
        println!("$ {program} {}", args.join(" "));
        let mut c = command_for(program, args)?;
        configure_env(&mut c, root, a, clean.path(), false);
        execute(c, timeout, false)?; Ok(())
    };
    let missing = find_program("pi").is_none();
    if missing {
        ensure!(a.update_pi, "Pi is missing and Pi installation was not authorized");
        run("npm", &["install", "-g", "--ignore-scripts", "@earendil-works/pi-coding-agent"])?;
        ensure!(find_program("pi").is_some(), "Pi installed but not discoverable; fix PATH, then rerun update");
    }
    let mut failures = Vec::new();
    if a.update_pi && !missing {
        if let Err(e) = run("pi", &["update", "--self"]) { eprintln!("Pi update: {e:#}"); failures.push("Pi core".to_owned()); }
    }
    for s in sources {
        ensure!(!CANCELLED.load(std::sync::atomic::Ordering::SeqCst), "Cancelled; no further updates started");
        config::checked_source(s)?;
        let result = (|| -> Result<()> {
            let (_, name, _) = PACKAGES.iter().find(|(_, n, _)| config::matches_package(&serde_json::json!(s), n)).context("Unknown package")?;
            let rel = format!("npm/node_modules/{name}/package.json");
            let installed = storage::read_optional(root, &rel)?
                .map(|b| serde_json::from_slice::<serde_json::Value>(&b))
                .transpose()?;
            let version = installed.as_ref().and_then(|j| j["version"].as_str());
            if let Some(verb) = config::package_action(s, version)? { run("pi", &[verb, s])?; }
            else { println!("Already installed at selected version: {s}"); }
            ensure!(storage::read_optional(root, &rel)?.is_some(), "Package command returned without installing {name}");
            Ok(())
        })();
        if let Err(e) = result { eprintln!("{s}: {e:#}"); failures.push(s.clone()); }
    }
    ensure!(failures.is_empty(), "Configuration saved, but online updates failed: {}. No fallback route or unlimited retry used", failures.join(", "));
    Ok(())
}
pub fn doctor(root: &Path) -> Result<usize> {
    println!("Agent directory: {}", root.display());
    let mut problems = 0;
    if let Some(id) = storage::pending(root)? { eprintln!("FAIL incomplete transaction: {id}; use restore"); problems += 1; }
    let saved = storage::load(root)?;
    if let Some(saved) = &saved {
        match storage::plan(root, &saved.answers, false) {
            Ok(p) if p.changes.is_empty() => println!("OK managed configuration matches saved choices"),
            Ok(p) => { eprintln!("DRIFT {} managed files differ; review plan", p.changes.len()); problems += 1; },
            Err(e) => { eprintln!("FAIL {e:#}"); problems += 1; },
        }
    } else { println!("INFO no Rust profile state yet; run configure"); problems += 1; }
    match version("pi", root) { Ok(v) => println!("Pi: {v}"), Err(e) => { eprintln!("FAIL {e:#}"); problems += 1; } }
    let settings = storage::read_json(root, "settings.json")?;
    if let Some(saved) = &saved {
        for (id, name, _) in PACKAGES {
            if !saved.answers.enabled(id) { continue; }
            let rel = format!("npm/node_modules/{name}/package.json");
            match storage::read_optional(root, &rel)? {
                Some(b) => { let j: serde_json::Value = serde_json::from_slice(&b)?; println!("Installed {name}: {} (package presence, not a live extension test)", j["version"].as_str().unwrap_or("unknown")); },
                None => { eprintln!("MISSING {name} package files; declaration alone is not installation"); problems += 1; },
            }
        }
        if saved.answers.network == "split" {
            for p in [&saved.answers.public_proxy, &saved.answers.private_proxy] {
                println!("Proxy listener {p}: {} (TCP only)", proxy_available(p).unwrap_or(false));
            }
        }
    }
    if cfg!(windows) { println!("Git Bash: {}", git_bash().map(|p| p.display().to_string()).unwrap_or_else(|| "not found".into())); }
    for cmd in ["git", "rg", "fd", "uv", "ty", "ruff", "rustup", "rust-analyzer"] {
        if find_program(cmd).is_none() { println!("OPTIONAL missing: {cmd}"); }
    }
    if storage::read_optional(root, "pi-lsp.json")?.is_some() { println!("WARN legacy pi-lsp.json exists; kept untouched"); }
    // Authentication content is never opened.
    println!("auth.json present: {} (not read; login readiness not tested)", root.join("auth.json").is_file());
    if settings.get("defaultModel").is_some() { println!("Model availability is not verified; check /model in Pi after login."); }
    println!("Project overrides and real LSP diagnostics require validation inside each project.");
    Ok(problems)
}
