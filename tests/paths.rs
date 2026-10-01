use pi_profile::{config::Answers, system};
use std::{fs, process::Command};

#[test]
fn external_process_paths_are_compatible_without_changing_storage_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("agent space 中文");
    fs::create_dir(&root).unwrap();
    let canonical = fs::canonicalize(&root).unwrap();
    let mut cmd = Command::new("unused");
    system::configure_env(&mut cmd, &canonical, &Answers::default(), &canonical, false);
    let actual = cmd
        .get_envs()
        .find(|(k, _)| *k == "PI_CODING_AGENT_DIR")
        .unwrap()
        .1
        .unwrap();
    assert_eq!(std::path::Path::new(actual), dunce::simplified(&canonical));
    assert_eq!(fs::canonicalize(actual).unwrap(), canonical);
    assert_eq!(
        cmd.get_current_dir().unwrap(),
        dunce::simplified(&canonical)
    );
}

#[cfg(windows)]
#[test]
fn reserved_windows_paths_are_not_blindly_rewritten() {
    // Windows reserves NUL and COM1, not the ordinary filename COM.
    // https://learn.microsoft.com/windows/win32/fileio/naming-a-file
    for name in [r"\\?\C:\NUL", r"\\?\C:\COM1"] {
        let reserved = std::path::Path::new(name);
        assert_eq!(dunce::simplified(reserved), reserved);
    }
}
