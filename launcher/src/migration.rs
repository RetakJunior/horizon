//! One-time, opt-in import of the previous launcher's settings and instance metadata.
use std::fs;
use std::path::{Path, PathBuf};

const IMPORT_MARKER: &str = "horizon-import-v1.done";
const IMPORT_FILES: [&str; 2] = ["settings.json", "instances.json"];

fn default_legacy_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("horizon-launcher")
}

/// Copies settings and instance metadata from the old default data folder without replacing any
/// REa files. Accounts, game folders, worlds, logs, and caches are deliberately excluded.
pub fn import_legacy_data() -> Result<usize, String> {
    import_from(&default_legacy_dir(), &crate::backend::data_dir())
}

fn import_from(source: &Path, destination: &Path) -> Result<usize, String> {
    let marker = destination.join(IMPORT_MARKER);
    if marker.exists() {
        return Ok(0);
    }
    if !source.is_dir() {
        return Ok(0);
    }

    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let mut copied = 0;
    for name in IMPORT_FILES {
        let from = source.join(name);
        let to = destination.join(name);
        if !from.is_file() || to.exists() {
            continue;
        }
        let staged = destination.join(format!("{name}.rea-import-{}", std::process::id()));
        fs::copy(&from, &staged).map_err(|e| e.to_string())?;
        if let Err(error) = fs::rename(&staged, &to) {
            let _ = fs::remove_file(&staged);
            return Err(error.to_string());
        }
        copied += 1;
    }
    if copied > 0 {
        fs::write(marker, b"settings and instance metadata imported\n")
            .map_err(|e| e.to_string())?;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::import_from;
    use std::fs;

    #[test]
    fn imports_only_settings_and_instance_metadata_once_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("old");
        let destination = temp.path().join("rea");
        fs::create_dir_all(source.join("logs")).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(source.join("settings.json"), r##"{"accent":"#00e5ff"}"##).unwrap();
        fs::write(source.join("instances.json"), r#"{"instances":{}}"#).unwrap();
        fs::write(source.join("accounts.json"), "must not import").unwrap();
        fs::write(source.join("logs/latest.log"), "must not import").unwrap();
        fs::write(
            destination.join("settings.json"),
            r#"{"game_args":"--keep"}"#,
        )
        .unwrap();

        assert_eq!(import_from(&source, &destination).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(destination.join("settings.json")).unwrap(),
            r#"{"game_args":"--keep"}"#
        );
        assert_eq!(
            fs::read_to_string(destination.join("instances.json")).unwrap(),
            r#"{"instances":{}}"#
        );
        assert!(!destination.join("accounts.json").exists());
        assert!(!destination.join("logs/latest.log").exists());
        assert_eq!(import_from(&source, &destination).unwrap(), 0);
    }
}
