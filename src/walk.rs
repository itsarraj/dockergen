use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// The names (not full paths) of every entry directly inside `dir` — the
/// real filesystem call `detect::detect_stack` is deliberately kept
/// ignorant of, so detection stays testable with a plain string list.
pub fn list_top_level_names(dir: &Path) -> Result<Vec<String>> {
    let entries =
        fs::read_dir(dir).with_context(|| format!("reading directory {}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

pub fn read_cargo_toml(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join("Cargo.toml")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_real_directory_entries() {
        let dir = std::env::temp_dir().join(format!("dockergen-test-walk-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();

        let names = list_top_level_names(&dir).unwrap();
        assert!(names.contains(&"Cargo.toml".to_string()));
        assert!(names.contains(&"src".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_cargo_toml_returns_none_when_absent() {
        let dir =
            std::env::temp_dir().join(format!("dockergen-test-nocargo-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(read_cargo_toml(&dir), None);
        fs::remove_dir_all(&dir).ok();
    }
}
