use pi_profile::{
    config::{self, Answers, Mode},
    storage::{self, Change},
    system,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn temp() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = fs::canonicalize(d.path())
        .unwrap()
        .join("profile space 中文");
    fs::create_dir(&p).unwrap();
    (d, p)
}
fn apply(p: &Path, a: &Answers) -> Option<String> {
    let plan = storage::plan(p, a, false).unwrap();
    let _lock = storage::lock(p).unwrap();
    storage::commit(p, &plan.changes, false).unwrap()
}
#[test]
fn defaults_are_valid() {
    Answers::default().validate().unwrap();
}
#[test]
fn unknown_input_is_rejected() {
    assert!(serde_json::from_value::<Answers>(json!({"apiKey":"NOT-A-REAL-SECRET"})).is_err());
}
#[test]
fn unknown_schema_is_rejected() {
    let a = Answers {
        schema: 2,
        ..Answers::default()
    };
    assert!(a.validate().is_err());
}
#[test]
fn proxy_rejects_credentials_and_non_http() {
    for p in [
        "http://user:pass@localhost:10809",
        "http://localhost/path",
        "socks5://localhost:10809",
        "http://localhost/?token=x",
    ] {
        assert!(config::proxy_url(p).is_err());
    }
    config::proxy_url("http://127.0.0.1:10809").unwrap();
    config::proxy_url("http://[::1]:10809").unwrap();
}
#[test]
fn settings_preserve_unknowns_and_extensions() {
    let old = json!({"unrelated":{"x":7},"extensions":["/custom/tool.ts"],"httpProxy":"http://localhost:42","defaultTools":["read","+custom"]});
    let new = config::settings(&old, &Answers::default(), None).unwrap();
    assert_eq!(new["unrelated"], old["unrelated"]);
    assert_eq!(new["extensions"], old["extensions"]);
    assert_eq!(new["httpProxy"], old["httpProxy"]);
    assert_eq!(new["defaultTools"], json!(["read", "+custom", "+codemode"]));
}
#[test]
fn code_mode_disabled_via_tool_not_fake_off_mode() {
    let a = Answers {
        code_mode: false,
        ..Answers::default()
    };
    let s = config::settings(&json!({"defaultTools":["+codemode","+custom"]}), &a, None).unwrap();
    assert_eq!(s["defaultTools"], json!(["+custom", "-codemode"]));
    assert!(s.get("codemode").is_none());
}
#[test]
fn filters_and_pin_are_preserved() {
    let old = json!({"packages":[{"source":"npm:@ff-labs/pi-fff@0.11.0","extensions":["allowed.ts"]},"npm:unrelated@1.2.3"]});
    let p = config::packages(&old, &Answers::default()).unwrap();
    assert_eq!(p[0], old["packages"][0]);
    assert_eq!(p[1], old["packages"][1]);
}
#[test]
fn deliberate_unpin_preserves_filters() {
    let a = Answers {
        latest_packages: true,
        ..Answers::default()
    };
    let p = config::packages(
        &json!({"packages":[{"source":"npm:@ff-labs/pi-fff@0.11.0","extensions":[]}]}),
        &a,
    )
    .unwrap();
    assert_eq!(
        p[0],
        json!({"source":"npm:@ff-labs/pi-fff","extensions":[]})
    );
}
#[test]
fn disabled_package_leaves_unrelated_entries() {
    let mut a = Answers::default();
    a.packages.insert("fff".into(), Mode::Disable);
    let p = config::packages(&json!({"packages":["npm:@ff-labs/pi-fff","npm:other"]}), &a).unwrap();
    assert!(!p
        .iter()
        .any(|x| config::matches_package(x, "@ff-labs/pi-fff")));
    assert!(p.contains(&json!("npm:other")));
}
#[test]
fn duplicate_packages_are_not_silently_destroyed() {
    assert!(config::packages(
        &json!({"packages":["npm:@ff-labs/pi-fff","npm:@ff-labs/pi-fff@1"]}),
        &Answers::default()
    )
    .is_err());
}
#[test]
fn dynamic_rules_omit_disabled_features_and_routes() {
    let mut a = Answers {
        code_mode: false,
        network: "direct".into(),
        ..Answers::default()
    };
    a.packages.insert("prune".into(), Mode::Disable);
    a.packages.insert("compact".into(), Mode::Disable);
    let (global, runtime) = config::render_rules(&a);
    assert!(!global.contains("10809"));
    assert!(!runtime.contains("## Code Mode"));
    assert!(!runtime.contains("context_prune"));
}
#[test]
fn invalid_json_causes_zero_writes() {
    let (_d, p) = temp();
    fs::write(p.join("lsp.json"), "broken").unwrap();
    assert!(storage::plan(&p, &Answers::default(), false).is_err());
    assert!(!p.join("AGENTS.md").exists());
    assert!(!p.join(".pi-profile").exists());
}
#[test]
fn non_object_settings_rejected() {
    let (_d, p) = temp();
    fs::write(p.join("settings.json"), "[]").unwrap();
    assert!(storage::plan(&p, &Answers::default(), false).is_err());
}
#[test]
fn idempotent_apply_does_not_create_another_backup() {
    let (_d, p) = temp();
    let a = Answers::default();
    apply(&p, &a);
    assert!(storage::plan(&p, &a, false).unwrap().changes.is_empty());
    assert!(apply(&p, &a).is_none());
    assert_eq!(storage::backups(&p).unwrap().len(), 1);
}
#[test]
fn formatting_only_differences_are_not_drift() {
    let (_d, p) = temp();
    apply(&p, &Answers::default());
    let s = storage::read_json(&p, "settings.json").unwrap();
    fs::write(p.join("settings.json"), serde_json::to_vec(&s).unwrap()).unwrap();
    assert!(storage::plan(&p, &Answers::default(), false)
        .unwrap()
        .changes
        .is_empty());
}
#[test]
fn backup_restores_existing_and_removes_new_files() {
    let (_d, p) = temp();
    let original = b"{\"unrelated\":true}\n";
    fs::write(p.join("settings.json"), original).unwrap();
    let id = apply(&p, &Answers::default()).unwrap();
    let plan = storage::restore_plan(&p, &id, false).unwrap();
    storage::commit(&p, &plan, true).unwrap();
    assert_eq!(fs::read(p.join("settings.json")).unwrap(), original);
    assert!(!p.join("AGENTS.md").exists());
    assert!(!p.join(storage::STATE).exists());
}
#[test]
fn outside_rule_block_preserved_inside_edit_rejected() {
    let (_d, p) = temp();
    fs::write(p.join("AGENTS.md"), "# User rules\nKeep my project data.\n").unwrap();
    apply(&p, &Answers::default());
    let file = p.join("AGENTS.md");
    let mut text = fs::read_to_string(&file).unwrap();
    text.push_str("\nPersonal footer.\n");
    fs::write(&file, &text).unwrap();
    assert!(storage::plan(&p, &Answers::default(), false)
        .unwrap()
        .changes
        .is_empty());
    fs::write(
        &file,
        text.replace("## Ownership and scope", "## User-edited scope"),
    )
    .unwrap();
    assert!(storage::plan(&p, &Answers::default(), false).is_err());
    assert!(storage::plan(&p, &Answers::default(), true).is_ok());
}
#[test]
fn malformed_rule_markers_fail_closed() {
    assert!(
        storage::replace_block("<!-- pi-profile:begin -->", Some("hi"), None, false, "").is_err()
    );
}
#[test]
fn existing_override_blocks_only_rules_installation() {
    let (_d, p) = temp();
    fs::write(p.join("AGENTS.override.md"), "Protected").unwrap();
    assert!(storage::plan(&p, &Answers::default(), false).is_err());
    let a = Answers {
        rules: false,
        ..Answers::default()
    };
    assert!(storage::plan(&p, &a, false).is_ok());
}
#[test]
fn concurrent_edit_rejected_before_writes() {
    let (_d, p) = temp();
    let plan = storage::plan(&p, &Answers::default(), false).unwrap();
    fs::write(p.join("settings.json"), "{\"new\":1}").unwrap();
    assert!(storage::commit(&p, &plan.changes, false).is_err());
    assert!(!p.join("AGENTS.md").exists());
}
#[test]
fn restore_refuses_post_install_edit_without_force() {
    let (_d, p) = temp();
    let id = apply(&p, &Answers::default()).unwrap();
    fs::write(p.join("AGENTS.md"), "new personal edits").unwrap();
    assert!(storage::restore_plan(&p, &id, false).is_err());
    assert!(storage::restore_plan(&p, &id, true).is_ok());
}
#[test]
fn corrupted_backup_rejected_even_with_force() {
    let (_d, p) = temp();
    fs::write(p.join("AGENTS.md"), "original").unwrap();
    let id = apply(&p, &Answers::default()).unwrap();
    fs::write(
        p.join(format!(".pi-profile/backups/{id}/files/AGENTS.md")),
        "tampered",
    )
    .unwrap();
    assert!(storage::restore_plan(&p, &id, true).is_err());
}
#[test]
fn interrupted_journal_requires_recovery() {
    let (_d, p) = temp();
    let id = apply(&p, &Answers::default()).unwrap();
    fs::write(
        p.join(".pi-profile/pending.json"),
        serde_json::to_vec(&id).unwrap(),
    )
    .unwrap();
    let c = Change {
        path: "AGENTS.md".into(),
        before: Some(fs::read(p.join("AGENTS.md")).unwrap()),
        after: Some(b"other".to_vec()),
    };
    assert!(storage::commit(&p, &[c], false).is_err());
    let plan = storage::restore_plan(&p, &id, false).unwrap();
    storage::commit(&p, &plan, true).unwrap();
    assert!(storage::pending(&p).unwrap().is_none());
}
#[test]
fn paths_and_manifest_targets_are_bounded() {
    let (_d, p) = temp();
    for bad in [
        "../auth.json",
        "/tmp/escape",
        ".pi-profile/../auth.json",
        "a\\b",
    ] {
        assert!(storage::safe_path(&p, bad).is_err());
    }
    let c = Change {
        path: "auth.json".into(),
        before: None,
        after: Some(b"secret".to_vec()),
    };
    assert!(storage::commit(&p, &[c], false).is_err());
}
#[test]
fn lock_excludes_second_writer() {
    let (_d, p) = temp();
    let first = storage::lock(&p).unwrap();
    assert!(storage::lock(&p).is_err());
    drop(first);
    assert!(storage::lock(&p).is_ok());
}
#[cfg(unix)]
#[test]
fn symlink_parent_is_rejected() {
    use std::os::unix::fs::symlink;
    let (_d, p) = temp();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), p.join("extensions")).unwrap();
    assert!(storage::plan(&p, &Answers::default(), false).is_err());
    assert!(!outside.path().join("pi-better-compaction").exists());
}
#[test]
fn node_version_boundary() {
    assert!(!system::node_supported("v22.18.9"));
    assert!(system::node_supported("v22.19.0"));
    assert!(system::node_supported("v24.0.0"));
    assert!(!system::node_supported("nonsense"));
}
#[test]
fn source_validation_rejects_shell_payloads() {
    assert!(config::checked_source("npm:@ff-labs/pi-fff@latest;echo").is_err());
    assert!(config::checked_source("npm:other").is_err());
    config::checked_source("npm:@ff-labs/pi-fff@0.11.0").unwrap();
}
fn cli(p: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pi-profile"));
    c.arg("--agent-dir").arg(p).stdin(Stdio::null());
    c
}
#[test]
fn noninteractive_start_requires_explicit_answers() {
    let (_d, p) = temp();
    let out = cli(&p).output().unwrap();
    assert!(!out.status.success());
    assert!(!p.join(storage::STATE).exists());
}
#[test]
fn offline_cli_and_dry_run_are_safe() {
    let (_d, p) = temp();
    let answers = p.join("answers.json");
    fs::write(&answers, storage::json_bytes(&Answers::default()).unwrap()).unwrap();
    let out = cli(&p)
        .args(["configure", "--yes", "--offline", "--dry-run", "--answers"])
        .arg(&answers)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!p.join(storage::STATE).exists());
    let out = cli(&p)
        .args(["configure", "--yes", "--offline", "--answers"])
        .arg(&answers)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(p.join(storage::STATE).exists());
    assert!(String::from_utf8_lossy(&out.stdout).contains("NOT performed"));
}
#[test]
fn exported_answers_do_not_copy_unknown_secrets() {
    let (_d, p) = temp();
    fs::write(
        p.join("settings.json"),
        "{\"mySecret\":\"SENTINEL_PRIVATE\"}",
    )
    .unwrap();
    apply(&p, &Answers::default());
    let output = p.join("export.json");
    let out = cli(&p)
        .args(["export", "--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = fs::read_to_string(output).unwrap();
    assert!(!text.contains("SENTINEL_PRIVATE"));
    let _: Answers = serde_json::from_str(&text).unwrap();
}
#[test]
fn process_runner_collects_small_version_output() {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pi-profile"));
    c.arg("--version");
    assert!(system::execute(c, std::time::Duration::from_secs(10), true)
        .unwrap()
        .contains("pi-profile"));
}
#[test]
fn profile_state_contains_only_choices_and_hashes() {
    let (_d, p) = temp();
    apply(&p, &Answers::default());
    let state: Value = serde_json::from_slice(&fs::read(p.join(storage::STATE)).unwrap()).unwrap();
    assert_eq!(state["schema"], 1);
    assert_eq!(state["blocks"]["AGENTS.md"].as_str().unwrap().len(), 64);
    assert!(state.get("auth").is_none());
}
