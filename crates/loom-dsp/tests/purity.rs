//! The crate depends on nothing that reaches its users (ADR-0035). Dev
//! dependencies are allowed, because they only build its tests.

/// True for a manifest section that adds a dependency for users of the crate.
fn adds_user_dependencies(section: &str) -> bool {
    let name = section.rsplit('.').next().unwrap_or(section);
    name == "dependencies" || name == "build-dependencies"
}

/// The entries of `manifest` that would give users a dependency.
fn user_dependencies(manifest: &str) -> Vec<String> {
    let mut section = String::new();
    let mut found = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[') {
            section = name.trim_end_matches(']').to_string();
            // `[dependencies.foo]` is an entry by itself.
            if section.contains("dependencies.") && !section.contains("dev-dependencies") {
                found.push(section.clone());
            }
        } else if adds_user_dependencies(&section) {
            found.push(line.to_string());
        }
    }
    found
}

/// The manifest lists no normal or build dependency, on any target.
#[test]
fn the_crate_has_no_user_dependencies() {
    assert_eq!(user_dependencies(include_str!("../Cargo.toml")), Vec::<String>::new());
}

/// The check above can fail: it sees each way to write a dependency, and
/// allows dev dependencies.
#[test]
fn the_check_sees_each_form() {
    assert_eq!(user_dependencies("[dependencies]\nfoo = \"1\"\n"), ["foo = \"1\""]);
    assert_eq!(
        user_dependencies("[build-dependencies]\nfoo = \"1\"\n"),
        ["foo = \"1\""]
    );
    assert_eq!(
        user_dependencies("[target.'cfg(unix)'.dependencies]\nfoo = \"1\"\n"),
        ["foo = \"1\""]
    );
    assert_eq!(
        user_dependencies("[dependencies.foo]\nversion = \"1\"\n"),
        ["dependencies.foo"]
    );
    assert!(user_dependencies("[dev-dependencies]\nfoo = \"1\"\n").is_empty());
    assert!(user_dependencies("[dev-dependencies.foo]\nversion = \"1\"\n").is_empty());
}
