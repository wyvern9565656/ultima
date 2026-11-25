use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Config, InstalledRecord, Persisted};

pub fn state_path(root: &Path) -> PathBuf {
    root.join("state.json")
}

pub fn ensure_layout(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root.join("apps"))?;
    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("cache"))?;
    Ok(())
}

pub fn load(root: &Path) -> Persisted {
    let path = state_path(root);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Persisted>(&text) {
            Ok(mut parsed) => {
                if parsed.config.repos.is_empty() {
                    parsed.config.repos = Config::default().repos;
                }
                if parsed.config.install_dir.as_os_str().is_empty() {
                    parsed.config.install_dir = Config::default().install_dir;
                }
                parsed
            }
            Err(err) => {
                eprintln!("ultima: estado ilegível em {}: {err}", path.display());
                Persisted::default()
            }
        },
        Err(_) => Persisted::default(),
    }
}

pub fn save(root: &Path, config: &Config, installs: &[InstalledRecord]) -> std::io::Result<()> {
    ensure_layout(root)?;
    let payload = Persisted {
        config: config.clone(),
        installs: installs.to_vec(),
    };
    let text = serde_json::to_string_pretty(&payload).map_err(std::io::Error::other)?;
    let target = state_path(root);
    let temp = target.with_extension("json.tmp");
    fs::write(&temp, text)?;
    fs::rename(temp, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta_do_estado() {
        let root = std::env::temp_dir().join(format!("ultima-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        ensure_layout(&root).unwrap();
        let mut config = Config::default();
        config.token = Some("x".into());
        config.install_dir = root.clone();
        let installs = vec![InstalledRecord {
            repo: "a/b".into(),
            version: "v1".into(),
            channel: crate::model::Channel::Source,
            path: root.join("apps/a"),
            binary: None,
            installed_at: 1,
            checksum: None,
        }];
        save(&root, &config, &installs).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.config.repos.len(), 4);
        assert_eq!(loaded.installs.len(), 1);
        assert_eq!(loaded.config.token.as_deref(), Some("x"));
        fs::remove_dir_all(&root).ok();
    }
}
