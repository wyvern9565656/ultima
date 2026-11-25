use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::archive::{find_binary, make_executable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildSystem {
    Cargo,
    Node,
    Python,
    Unknown,
}

impl BuildSystem {
    pub fn label(self) -> &'static str {
        match self {
            BuildSystem::Cargo => "cargo",
            BuildSystem::Node => "npm",
            BuildSystem::Python => "python",
            BuildSystem::Unknown => "desconhecido",
        }
    }
}

pub fn detect(dir: &Path) -> BuildSystem {
    if dir.join("Cargo.toml").exists() {
        BuildSystem::Cargo
    } else if dir.join("package.json").exists() {
        BuildSystem::Node
    } else if dir.join("pyproject.toml").exists()
        || dir.join("requirements.txt").exists()
        || has_python_entry(dir)
    {
        BuildSystem::Python
    } else {
        BuildSystem::Unknown
    }
}

fn has_python_entry(dir: &Path) -> bool {
    for candidate in ["main.py", "app.py", "src/main.py"] {
        if dir.join(candidate).exists() {
            return true;
        }
    }
    fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "py")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Clona ou atualiza o repositório no diretório de fontes.
pub fn sync_source(
    url: &str,
    branch: &str,
    dir: &Path,
    log: &mut dyn FnMut(&str, bool),
) -> Result<(), String> {
    if which("git").is_none() {
        return Err("git não encontrado no PATH".into());
    }
    if dir.join(".git").is_dir() {
        log(&format!("$ git -C {} fetch --depth 1 origin {branch}", dir.display()), false);
        run_streaming(
            "git",
            &["fetch", "--depth", "1", "origin", branch],
            dir,
            &[],
            log,
        )?;
        log("$ git reset --hard FETCH_HEAD", false);
        run_streaming("git", &["reset", "--hard", "FETCH_HEAD"], dir, &[], log)?;
        run_streaming("git", &["clean", "-fd"], dir, &[], log)?;
    } else {
        if dir.exists() {
            fs::remove_dir_all(dir)
                .map_err(|e| format!("não foi possível limpar {}: {e}", dir.display()))?;
        }
        if let Some(parent) = dir.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("criar {}: {e}", parent.display()))?;
        }
        log(&format!("$ git clone --depth 1 --branch {branch} {url}"), false);
        run_streaming(
            "git",
            &["clone", "--depth", "1", "--branch", branch, url, &dir.display().to_string()],
            dir.parent().unwrap_or(Path::new(".")),
            &[],
            log,
        )?;
    }
    Ok(())
}

pub fn run_build(
    system: BuildSystem,
    dir: &Path,
    log: &mut dyn FnMut(&str, bool),
) -> Result<(), String> {
    match system {
        BuildSystem::Cargo => {
            log("$ cargo build --release", false);
            run_streaming(
                "cargo",
                &["build", "--release", "--color=never"],
                dir,
                &[("CARGO_TERM_COLOR", "never")],
                log,
            )
        }
        BuildSystem::Node => {
            let has_lock = dir.join("package-lock.json").exists();
            if has_lock {
                log("$ npm ci", false);
                run_streaming("npm", &["ci", "--no-audit", "--no-fund"], dir, &[], log)?;
            } else {
                log("$ npm install", false);
                run_streaming("npm", &["install", "--no-audit", "--no-fund"], dir, &[], log)?;
            }
            if has_script(dir, "build") {
                log("$ npm run build", false);
                run_streaming("npm", &["run", "build"], dir, &[], log)?;
            }
            Ok(())
        }
        BuildSystem::Python => {
            log("python: nada para compilar, o pacote será copiado", false);
            Ok(())
        }
        BuildSystem::Unknown => Err(
            "sistema de build não reconhecido (esperado Cargo.toml, package.json ou Python)".into(),
        ),
    }
}

fn has_script(dir: &Path, script: &str) -> bool {
    let Ok(text) = fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    json.get("scripts")
        .and_then(|s| s.get(script))
        .map(|v| !v.is_null())
        .unwrap_or(false)
}

/// Executa um comando repassando cada linha de saída para `log`.
pub fn run_streaming(
    program: &str,
    args: &[&str],
    cwd: &Path,
    envs: &[(&str, &str)],
    log: &mut dyn FnMut(&str, bool),
) -> Result<(), String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("não foi possível executar `{program}`: {e}"))?;

    let (tx, rx) = mpsc::channel::<(String, bool)>();
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || pump(stdout, tx, false)));
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || pump(stderr, tx, true)));
    }
    drop(tx);

    for (line, is_error) in rx {
        log(&line, is_error);
    }
    for reader in readers {
        let _ = reader.join();
    }
    let status = child.wait().map_err(|e| format!("falha ao aguardar `{program}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program}` terminou com {}",
            status
                .code()
                .map(|c| format!("código {c}"))
                .unwrap_or_else(|| "sinal".into())
        ))
    }
}

fn pump<R: Read>(reader: R, tx: mpsc::Sender<(String, bool)>, is_error: bool) {
    let mut lines = BufReader::new(reader).lines();
    while let Some(Ok(line)) = lines.next() {
        if tx.send((line, is_error)).is_err() {
            break;
        }
    }
}

fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// Nome do pacote declarado no Cargo.toml, usado para localizar o binário.
pub fn cargo_package_name(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = line.strip_prefix("name") {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix('=') {
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
            if line.starts_with("[[bin]]") {
                break;
            }
        }
    }
    None
}

/// Localiza o executável produzido pelo cargo em `target/release`.
pub fn find_cargo_binary(dir: &Path, hints: &[&str]) -> Option<PathBuf> {
    let release = dir.join("target").join("release");
    if !release.is_dir() {
        return None;
    }
    find_binary(&release, hints, 2).or_else(|| find_binary(&release, hints, 2))
}

/// Copia o fonte Python e cria um launcher executável.
pub fn package_python(
    repo_dir: &Path,
    install_dir: &Path,
    slug: &str,
    log: &mut dyn FnMut(&str, bool),
) -> Result<Option<PathBuf>, String> {
    fs::create_dir_all(install_dir).map_err(|e| format!("criar {}: {e}", install_dir.display()))?;
    copy_tree(repo_dir, install_dir, 0, log)?;

    let entry = find_python_entry(install_dir).ok_or_else(|| {
        "não encontrei um ponto de entrada Python (main.py, app.py ou bloco __main__)".to_string()
    })?;
    let launcher = install_dir.join(format!("{slug}.sh"));
    let script = format!(
        "#!/bin/sh\ncd \"$(dirname \"$0\")\" || exit 1\nexec python3 \"{}\" \"$@\"\n",
        entry
    );
    fs::write(&launcher, script).map_err(|e| format!("criar launcher: {e}"))?;
    make_executable(&launcher);
    log(&format!("launcher criado: {}", launcher.display()), false);
    Ok(Some(launcher))
}

/// Copia o fonte Node (já construído) e cria um launcher com `npm start`.
pub fn package_node(
    repo_dir: &Path,
    install_dir: &Path,
    slug: &str,
    log: &mut dyn FnMut(&str, bool),
) -> Result<Option<PathBuf>, String> {
    fs::create_dir_all(install_dir).map_err(|e| format!("criar {}: {e}", install_dir.display()))?;
    copy_tree(repo_dir, install_dir, 0, log)?;
    let launcher = install_dir.join(format!("{slug}.sh"));
    let command = if has_script(install_dir, "start") {
        "npm start".to_string()
    } else {
        let entry = fs::read_to_string(install_dir.join("package.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|json| json.get("main").and_then(|m| m.as_str()).map(str::to_string))
            .unwrap_or_else(|| "index.js".into());
        format!("node \"{entry}\"")
    };
    let script = format!(
        "#!/bin/sh\ncd \"$(dirname \"$0\")\" || exit 1\nexec {command} \"$@\"\n"
    );
    fs::write(&launcher, script).map_err(|e| format!("criar launcher: {e}"))?;
    make_executable(&launcher);
    log(&format!("launcher criado: {}", launcher.display()), false);
    Ok(Some(launcher))
}

fn find_python_entry(dir: &Path) -> Option<String> {
    for candidate in ["main.py", "app.py", "src/main.py"] {
        if dir.join(candidate).is_file() {
            return Some(candidate.to_string());
        }
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "py").unwrap_or(false) {
            if let Ok(text) = fs::read_to_string(&path) {
                if text.contains("__main__") {
                    return Some(
                        path.strip_prefix(dir)
                            .unwrap_or(&path)
                            .display()
                            .to_string(),
                    );
                }
            }
        }
    }
    None
}

fn copy_tree(
    from: &Path,
    to: &Path,
    depth: usize,
    log: &mut dyn FnMut(&str, bool),
) -> Result<(), String> {
    if depth > 12 {
        return Ok(());
    }
    let entries = match fs::read_dir(from) {
        Ok(entries) => entries,
        Err(err) => {
            log(&format!("aviso: não foi possível ler {}: {err}", from.display()), true);
            return Ok(());
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if matches!(
            name_str.as_ref(),
            ".git" | "__pycache__" | "node_modules" | ".venv" | "venv" | "target" | ".mypy_cache"
        ) || name_str.ends_with(".pyc")
        {
            continue;
        }
        let src = entry.path();
        let dst = to.join(&name);
        if src.is_dir() {
            fs::create_dir_all(&dst).map_err(|e| format!("criar {}: {e}", dst.display()))?;
            copy_tree(&src, &dst, depth + 1, log)?;
        } else {
            fs::copy(&src, &dst)
                .map_err(|e| format!("copiar {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_sistemas() {
        let root = std::env::temp_dir().join(format!("ultima-detect-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert_eq!(detect(&root), BuildSystem::Unknown);
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(detect(&root), BuildSystem::Cargo);
        fs::remove_file(root.join("Cargo.toml")).unwrap();
        fs::write(root.join("main.py"), "print('oi')").unwrap();
        assert_eq!(detect(&root), BuildSystem::Python);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn le_nome_do_pacote_cargo() {
        let root = std::env::temp_dir().join(format!("ultima-cargo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"ultima-pulse\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
        )
        .unwrap();
        assert_eq!(cargo_package_name(&root).as_deref(), Some("ultima-pulse"));
        fs::remove_dir_all(&root).ok();
    }
}

