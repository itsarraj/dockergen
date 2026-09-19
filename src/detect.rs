#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stack {
    Rust,
    NodeNpm,
    NodeYarn,
    NodePnpm,
    NodeBun,
    Python,
    Go,
}

impl Stack {
    pub fn name(&self) -> &'static str {
        match self {
            Stack::Rust => "Rust",
            Stack::NodeNpm => "Node (npm)",
            Stack::NodeYarn => "Node (yarn)",
            Stack::NodePnpm => "Node (pnpm)",
            Stack::NodeBun => "Node (bun)",
            Stack::Python => "Python",
            Stack::Go => "Go",
        }
    }
}

/// Detects a project's stack from the *names* of files present at its
/// root — pure and given a plain file-name list rather than a directory
/// path, so it's testable without touching a filesystem at all; the real
/// directory read is one line in `walk.rs`. Checked in a specific,
/// deliberate order: `Cargo.toml`/`go.mod` are unambiguous single-purpose
/// marker files, checked first; `package.json` needs a second look at
/// which lockfile (if any) is present to know the package manager, so
/// it's checked after; a Python project has two different valid marker
/// files (`pyproject.toml`, the modern one, or `requirements.txt`, the
/// older one) and neither implies anything about a package manager the
/// way Node's lockfiles do.
pub fn detect_stack(files: &[String]) -> Option<Stack> {
    let has = |name: &str| files.iter().any(|f| f == name);

    if has("Cargo.toml") {
        return Some(Stack::Rust);
    }
    if has("go.mod") {
        return Some(Stack::Go);
    }
    if has("package.json") {
        if has("bun.lock") || has("bun.lockb") {
            return Some(Stack::NodeBun);
        }
        if has("pnpm-lock.yaml") {
            return Some(Stack::NodePnpm);
        }
        if has("yarn.lock") {
            return Some(Stack::NodeYarn);
        }
        return Some(Stack::NodeNpm); // package-lock.json, or no lockfile at all
    }
    if has("pyproject.toml") || has("requirements.txt") {
        return Some(Stack::Python);
    }
    None
}

/// Pulls `name = "..."` out of a `Cargo.toml`'s `[package]` section —
/// enough to fill the binary name into the generated Dockerfile's final
/// `COPY --from=build` line without pulling in a full TOML parser for one
/// field. Deliberately simple: stops looking once it leaves `[package]`
/// (so a `[dependencies]` section that happens to have its own `name =`
/// line, unlikely but possible for a renamed dependency, can't be
/// mistaken for the package's own name).
pub fn parse_cargo_package_name(cargo_toml: &str) -> Option<String> {
    let mut in_package_section = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
            continue;
        }
        if in_package_section {
            if let Some(rest) = trimmed.strip_prefix("name") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let value = rest.trim();
                    let unquoted = value.trim_matches('"').trim_matches('\'');
                    if !unquoted.is_empty() {
                        return Some(unquoted.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn detects_rust_from_cargo_toml() {
        assert_eq!(
            detect_stack(&files(&["Cargo.toml", "src"])),
            Some(Stack::Rust)
        );
    }

    #[test]
    fn detects_go_from_go_mod() {
        assert_eq!(
            detect_stack(&files(&["go.mod", "main.go"])),
            Some(Stack::Go)
        );
    }

    #[test]
    fn node_package_manager_detected_from_lockfile() {
        assert_eq!(
            detect_stack(&files(&["package.json", "bun.lock"])),
            Some(Stack::NodeBun)
        );
        assert_eq!(
            detect_stack(&files(&["package.json", "pnpm-lock.yaml"])),
            Some(Stack::NodePnpm)
        );
        assert_eq!(
            detect_stack(&files(&["package.json", "yarn.lock"])),
            Some(Stack::NodeYarn)
        );
        assert_eq!(
            detect_stack(&files(&["package.json", "package-lock.json"])),
            Some(Stack::NodeNpm)
        );
    }

    #[test]
    fn node_with_no_lockfile_at_all_defaults_to_npm() {
        assert_eq!(
            detect_stack(&files(&["package.json"])),
            Some(Stack::NodeNpm)
        );
    }

    #[test]
    fn python_detected_from_either_marker_file() {
        assert_eq!(
            detect_stack(&files(&["pyproject.toml"])),
            Some(Stack::Python)
        );
        assert_eq!(
            detect_stack(&files(&["requirements.txt"])),
            Some(Stack::Python)
        );
    }

    #[test]
    fn rust_takes_priority_when_multiple_markers_somehow_coexist() {
        // A Rust project vendoring a Node-based frontend tool, say — the
        // ordering itself is the documented, deliberate policy above.
        assert_eq!(
            detect_stack(&files(&["Cargo.toml", "package.json"])),
            Some(Stack::Rust)
        );
    }

    #[test]
    fn no_recognized_marker_files_is_none() {
        assert_eq!(detect_stack(&files(&["README.md", "LICENSE"])), None);
    }

    #[test]
    fn parses_package_name_from_a_realistic_cargo_toml() {
        let toml = "[package]\nname = \"pgqueue\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nname = \"not-this-one\"\n";
        assert_eq!(parse_cargo_package_name(toml), Some("pgqueue".to_string()));
    }

    #[test]
    fn missing_package_section_yields_none() {
        assert_eq!(
            parse_cargo_package_name("[dependencies]\nserde = \"1\"\n"),
            None
        );
    }
}
