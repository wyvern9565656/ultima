use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;

use crate::archive::{self, download, extract, find_binary, flatten_single_root, make_executable};
use crate::build::{self, BuildSystem};
use crate::github::Github;
use crate::model::{now_epoch, parse_checksum_file, Asset, Channel, InstalledRecord, Release, RepoInfo, RepoRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

pub enum Event {
    Log {
        job: u64,
        level: LogLevel,
        line: String,
    },
    Phase {
        job: u64,
        phase: String,
        frac: Option<f32>,
    },
    Done {
        job: u64,
        record: Option<InstalledRecord>,
        error: Option<String>,
    },
    Repo {
        repo: String,
        result: Result<RepoInfo, String>,
    },
    Releases {
        repo: String,
        result: Result<Vec<Release>, String>,
    },
    Head {
        repo: String,
        result: Result<(String, String), String>,
    },
    Discover {
        result: Result<Vec<RepoInfo>, String>,
    },
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

fn hint_list(repo: &RepoRef, extra: Option<String>) -> Vec<String> {
    let mut hints = vec![
        repo.name().to_lowercase(),
        repo.title().to_lowercase().replace(' ', ""),
        repo.title().to_lowercase().replace(' ', "-"),
    ];
    if let Some(extra) = extra {
        hints.push(extra.to_lowercase());
    }
    hints.sort();
    hints.dedup();
    hints
}

// ---------------------------------------------------------------------------
// Metadados
// ---------------------------------------------------------------------------

pub fn spawn_repo_refresh(tx: Sender<Event>, token: Option<String>, repo: RepoRef) {
    thread::spawn(move || {
        let client = Github::new(token);
        let id = repo.full_name.clone();
        match client.fetch_repo(&id) {
            Ok(info) => {
                let branch = info.default_branch.clone();
                let _ = tx.send(Event::Repo {
                    repo: id.clone(),
                    result: Ok(info),
                });
                match client.fetch_releases(&id) {
                    Ok(releases) => {
                        let _ = tx.send(Event::Releases {
                            repo: id.clone(),
                            result: Ok(releases),
                        });
                    }
                    Err(err) => {
                        let _ = tx.send(Event::Releases {
                            repo: id.clone(),
                            result: Err(err),
                        });
                    }
                }
                let head = client.head_commit(&id, &branch);
                let _ = tx.send(Event::Head { repo: id, result: head });
            }
            Err(err) => {
                let _ = tx.send(Event::Repo {
                    repo: id.clone(),
                    result: Err(err.clone()),
                });
                let _ = tx.send(Event::Releases {
                    repo: id,
                    result: Err(err),
                });
            }
        }
    });
}

pub fn spawn_discover(tx: Sender<Event>, token: Option<String>, login: String) {
    thread::spawn(move || {
        let client = Github::new(token);
        let result = client.fetch_user_repos(&login);
        let _ = tx.send(Event::Discover { result });
    });
}

// ---------------------------------------------------------------------------
// Instalação de release
// ---------------------------------------------------------------------------

pub struct InstallRequest {
    pub job: u64,
    pub repo: RepoRef,
    pub release: Release,
    pub asset: Asset,
    pub checksum_url: Option<String>,
}

pub fn spawn_install(tx: Sender<Event>, token: Option<String>, root: PathBuf, request: InstallRequest) {
    thread::spawn(move || {
        let job = request.job;
        let result = run_install(&tx, job, token.as_deref(), &root, &request);
        match result {
            Ok(record) => {
                let _ = tx.send(Event::Done {
                    job,
                    record: Some(record),
                    error: None,
                });
            }
            Err(err) => {
                send_log(&tx, job, LogLevel::Error, &err);
                let _ = tx.send(Event::Done {
                    job,
                    record: None,
                    error: Some(err),
                });
            }
        }
    });
}

fn run_install(
    tx: &Sender<Event>,
    job: u64,
    token: Option<&str>,
    root: &Path,
    request: &InstallRequest,
) -> Result<InstalledRecord, String> {
    let repo = &request.repo;
    let asset = &request.asset;
    let slug = repo.slug();
    let version = request.release.tag.clone();
    let version_dir = sanitize(&version);

    let cache_path = root.join("cache").join(format!("{}-{}", slug, sanitize(&asset.name)));
    emit_phase(tx, job, format!("baixando {}", asset.name), Some(0.0));

    let mut last_label = String::new();
    let mut progress = |done: u64, total: u64| {
        let frac = if total > 0 {
            Some((done as f32 / total as f32).clamp(0.0, 1.0))
        } else {
            None
        };
        let label = format!("baixando {} · {}", asset.name, crate::model::human_size(done));
        if label != last_label {
            last_label = label.clone();
            let _ = tx.send(Event::Phase { job, phase: label, frac });
        }
    };

    let outcome = download(&asset.url, &cache_path, token, &mut progress)?;
    send_log(
        tx,
        job,
        LogLevel::Info,
        &format!("{} baixado ({})", asset.name, crate::model::human_size(outcome.bytes)),
    );

    // Checksum: digest da API ou arquivo de checksums publicado junto do release.
    let mut expected = asset.digest.as_deref().map(archive::normalize_sha);
    if expected.is_none() {
        if let Some(url) = &request.checksum_url {
            emit_phase(tx, job, "buscando checksums do release", None);
            match archive::fetch_text(url, token) {
                Ok(text) => {
                    if let Some(hash) = parse_checksum_file(&text, &asset.name) {
                        expected = Some(hash);
                    }
                }
                Err(err) => send_log(tx, job, LogLevel::Warn, &format!("aviso: {err}")),
            }
        }
    }

    let mut verified: Option<String> = None;
    match &expected {
        Some(expected) => {
            archive::verify_sha256(expected, &outcome.sha256)?;
            send_log(
                tx,
                job,
                LogLevel::Info,
                &format!("checksum sha256 ✓ {}…", &outcome.sha256[..12]),
            );
            verified = Some(outcome.sha256.clone());
        }
        None => send_log(
            tx,
            job,
            LogLevel::Warn,
            "este release não publica checksum — integridade não verificada",
        ),
    }

    emit_phase(tx, job, format!("extraindo {}", asset.name), None);
    let apps = root.join("apps");
    let staging = apps.join(&slug).join(format!(".{version_dir}.partial"));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| format!("limpar staging: {e}"))?;
    }
    extract(&cache_path, &staging)?;
    flatten_single_root(&staging);

    let hints = hint_list(repo, None);
    let hint_refs: Vec<&str> = hints.iter().map(String::as_str).collect();
    let binary = find_binary(&staging, &hint_refs, 8);
    if let Some(bin) = &binary {
        make_executable(bin);
        send_log(
            tx,
            job,
            LogLevel::Info,
            &format!(
                "executável: {}",
                bin.strip_prefix(&staging).unwrap_or(bin).display()
            ),
        );
    } else {
        send_log(
            tx,
            job,
            LogLevel::Warn,
            "nenhum executável encontrado no pacote; os arquivos foram instalados mesmo assim",
        );
    }

    emit_phase(tx, job, "instalando", Some(1.0));
    let final_dir = apps.join(&slug).join(&version_dir);
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir).map_err(|e| format!("substituir instalação anterior: {e}"))?;
    }
    if let Some(parent) = final_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("criar {}: {e}", parent.display()))?;
    }
    fs::rename(&staging, &final_dir)
        .or_else(|_| copy_dir(&staging, &final_dir))
        .map_err(|e| format!("não foi possível mover a instalação: {e}"))?;
    let _ = fs::remove_dir_all(&staging);

    let binary = binary.map(|bin| {
        let rel = bin.strip_prefix(&staging).unwrap_or(&bin).to_path_buf();
        let rebased = final_dir.join(rel);
        make_executable(&rebased);
        rebased
    });

    let _ = fs::remove_file(&cache_path);

    let record = InstalledRecord {
        repo: repo.full_name.clone(),
        version,
        channel: Channel::Release,
        path: final_dir,
        binary,
        installed_at: now_epoch(),
        checksum: verified,
    };
    send_log(
        tx,
        job,
        LogLevel::Info,
        &format!("instalado em {}", record.path.display()),
    );
    Ok(record)
}

// ---------------------------------------------------------------------------
// Compilação a partir do código
// ---------------------------------------------------------------------------

pub fn spawn_compile(
    tx: Sender<Event>,
    token: Option<String>,
    root: PathBuf,
    job: u64,
    repo: RepoRef,
    branch: String,
) {
    thread::spawn(move || {
        let result = run_compile(&tx, job, token.as_deref(), &root, &repo, &branch);
        match result {
            Ok(record) => {
                let _ = tx.send(Event::Done {
                    job,
                    record: Some(record),
                    error: None,
                });
            }
            Err(err) => {
                send_log(&tx, job, LogLevel::Error, &err);
                let _ = tx.send(Event::Done {
                    job,
                    record: None,
                    error: Some(err),
                });
            }
        }
    });
}

fn run_compile(
    tx: &Sender<Event>,
    job: u64,
    _token: Option<&str>,
    root: &Path,
    repo: &RepoRef,
    branch: &str,
) -> Result<InstalledRecord, String> {
    let slug = repo.slug();
    let src_dir = root.join("src").join(&slug);

    emit_phase(tx, job, format!("sincronizando {branch}"), None);
    let mut log = |line: &str, stderr: bool| {
        send_log(
            tx,
            job,
            level_for(line, stderr),
            line,
        );
    };
    build::sync_source(&repo.clone_url(), branch, &src_dir, &mut log)?;

    let system = build::detect(&src_dir);
    emit_phase(
        tx,
        job,
        format!("compilando · {}", system.label()),
        None,
    );
    build::run_build(system, &src_dir, &mut log)?;

    emit_phase(tx, job, "instalando", Some(0.94));
    let apps = root.join("apps");
    let install_dir = apps.join(&slug).join("head");
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|e| format!("limpar instalação anterior: {e}"))?;
    }
    fs::create_dir_all(&install_dir).map_err(|e| format!("criar {}: {e}", install_dir.display()))?;

    let binary = match system {
        BuildSystem::Cargo => {
            let package = build::cargo_package_name(&src_dir);
            let hints = hint_list(repo, package.clone());
            let hint_refs: Vec<&str> = hints.iter().map(String::as_str).collect();
            let found = build::find_cargo_binary(&src_dir, &hint_refs).ok_or_else(|| {
                "build concluído, mas nenhum binário encontrado em target/release".to_string()
            })?;
            let name = found
                .file_name()
                .map(|n| n.to_owned())
                .unwrap_or_default();
            let dest = install_dir.join(&name);
            fs::copy(&found, &dest)
                .map_err(|e| format!("copiar binário para {}: {e}", dest.display()))?;
            make_executable(&dest);
            Some(dest)
        }
        BuildSystem::Python => build::package_python(&src_dir, &install_dir, &slug, &mut log)?,
        BuildSystem::Node => build::package_node(&src_dir, &install_dir, &slug, &mut log)?,
        BuildSystem::Unknown => return Err("sistema de build não reconhecido".into()),
    };

    emit_phase(tx, job, "concluído", Some(1.0));
    let record = InstalledRecord {
        repo: repo.full_name.clone(),
        version: branch.to_string(),
        channel: Channel::Source,
        path: install_dir,
        binary,
        installed_at: now_epoch(),
        checksum: None,
    };
    send_log(
        tx,
        job,
        LogLevel::Info,
        &format!("instalado em {}", record.path.display()),
    );
    Ok(record)
}

// ---------------------------------------------------------------------------
// Auxiliares
// ---------------------------------------------------------------------------

fn level_for(line: &str, stderr: bool) -> LogLevel {
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("erro:") || lower.contains("failed") {
        LogLevel::Error
    } else if stderr && lower.contains("warning") {
        LogLevel::Warn
    } else {
        LogLevel::Info
    }
}

fn send_log(tx: &Sender<Event>, job: u64, level: LogLevel, line: &str) {
    let _ = tx.send(Event::Log {
        job,
        level,
        line: line.to_string(),
    });
}

fn emit_phase(tx: &Sender<Event>, job: u64, phase: impl Into<String>, frac: Option<f32>) {
    let _ = tx.send(Event::Phase {
        job,
        phase: phase.into(),
        frac,
    });
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
