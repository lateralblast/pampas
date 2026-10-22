//! The version must match in Cargo.toml, README.md and the newest CHANGELOG.md heading.

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
}
