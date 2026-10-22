//! The version must match in Cargo.toml, README.md, the newest CHANGELOG.md heading and the GUI app's
//! `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` and `ui/package.json`.

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn versions_agree() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(read("README.md").contains(&format!("Version: {v}")), "README.md Version line");
    let changelog = read("CHANGELOG.md");
    let newest = changelog
        .lines()
        .find(|l| l.starts_with("## [") && !l.starts_with("## [Unreleased]"))
        .expect("a versioned CHANGELOG heading");
    assert!(newest.starts_with(&format!("## [{v}]")), "CHANGELOG newest heading: {newest}");
    assert!(
        read("src-tauri/Cargo.toml").contains(&format!("\nversion = \"{v}\"\n")),
        "src-tauri/Cargo.toml version"
    );
    assert!(read("ui/package.json").contains(&format!("\"version\": \"{v}\"")), "ui/package.json version");
    assert!(
        read("src-tauri/tauri.conf.json").contains(&format!("\"version\": \"{v}\"")),
        "src-tauri/tauri.conf.json version"
    );
}
