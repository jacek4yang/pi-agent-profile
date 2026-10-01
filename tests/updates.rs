use pi_profile::{config, system};
use std::{collections::BTreeMap, process::Command};

#[test]
fn missing_pin_is_installed_not_skipped_by_update() {
    assert_eq!(
        config::package_action("npm:pi-context-usage@2.1.0", None).unwrap(),
        Some("install")
    );
}
#[test]
fn matching_pin_is_not_reinstalled() {
    assert_eq!(
        config::package_action("npm:pi-context-usage@2.1.0", Some("2.1.0")).unwrap(),
        None
    );
}
#[test]
fn changed_pin_is_installed() {
    assert_eq!(
        config::package_action("npm:pi-context-usage@2.1.0", Some("2.0.0")).unwrap(),
        Some("install")
    );
}
#[test]
fn unpinned_missing_installs_and_existing_updates() {
    assert_eq!(
        config::package_action("npm:@ff-labs/pi-fff", None).unwrap(),
        Some("install")
    );
    assert_eq!(
        config::package_action("npm:@ff-labs/pi-fff", Some("0.11.0")).unwrap(),
        Some("update")
    );
}
#[test]
fn aliases_and_external_urls_are_not_public_catalogue_packages() {
    for s in [
        "npm:pi-context-usage@npm:private",
        "npm:pi-context-usage@https://example.test/a",
        "npm:pi-context-usage@latest;echo",
    ] {
        assert!(config::checked_source(s).is_err());
    }
}
#[test]
fn package_routes_override_bypass_and_provider_proxy_per_process() {
    let root = tempfile::tempdir().unwrap();
    let mut c = Command::new("unused");
    system::configure_env(
        &mut c,
        root.path(),
        &config::Answers::default(),
        root.path(),
        false,
    );
    let env: BTreeMap<_, _> = c
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert_eq!(env["HTTP_PROXY"].as_deref(), Some("http://127.0.0.1:10809"));
    assert_eq!(env["NO_PROXY"].as_deref(), Some("localhost,127.0.0.1,::1"));
    assert_eq!(env["PI_OFFLINE"], None);
}
#[test]
fn direct_mode_blocks_saved_provider_proxy_fallback() {
    let root = tempfile::tempdir().unwrap();
    let a = config::Answers {
        network: "direct".into(),
        ..Default::default()
    };
    let mut c = Command::new("unused");
    system::configure_env(&mut c, root.path(), &a, root.path(), false);
    let env: BTreeMap<_, _> = c
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert_eq!(env["HTTP_PROXY"].as_deref(), Some(""));
    assert_eq!(env["NO_PROXY"].as_deref(), Some("*"));
}
