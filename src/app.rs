use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::model::{
    display_tag, find_checksum_asset, is_newer, pick_asset, Channel, Config, InstalledRecord, Release,
    RepoInfo, RepoRef,
};
use crate::store;
use crate::theme;
use crate::ui;
use crate::worker::{self, Event, InstallRequest, LogLevel};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Library,
    Discover,
    Updates,
    Builds,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Installed,
    Updates,
    SourceOnly,
}

pub struct JobView {
    pub id: u64,
    pub repo: String,
    pub title: String,
    pub command: String,
    pub log: Vec<(LogLevel, String)>,
    pub phase: String,
    pub frac: Option<f32>,
    pub started: Instant,
    pub done: Option<Result<(), String>>,
    pub record: Option<InstalledRecord>,
}

impl JobView {
    pub fn elapsed(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    pub fn is_done(&self) -> bool {
        self.done.is_some()
    }
}

pub struct Toast {
    pub text: String,
    pub level: LogLevel,
    pub created: Instant,
}

pub struct App {
    pub root: PathBuf,
    pub config: Config,
    pub installs: HashMap<String, InstalledRecord>,
    pub infos: HashMap<String, RepoInfo>,
    pub releases: HashMap<String, Vec<Release>>,
    pub heads: HashMap<String, (String, String)>,
    pub errors: HashMap<String, String>,
    pub loading: HashSet<String>,
    pub selected: Option<String>,
    pub page: Page,
    pub filter: Filter,
    pub search: String,
    pub show_settings: bool,
    pub token_draft: String,
    pub dir_draft: String,
    pub new_repo_draft: String,
    pub settings_error: Option<String>,
    pub jobs: Vec<JobView>,
    pub next_job: u64,
    pub toasts: Vec<Toast>,
    pub discover: Option<Vec<RepoInfo>>,
    pub discover_loading: bool,
    pub discover_error: Option<String>,
    pub chosen: HashMap<String, String>,
    pub last_check: Option<Instant>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let default_root = crate::model::default_install_dir();
        let mut persisted = store::load(&default_root);
        if persisted.config.install_dir != default_root
            && store::state_path(&persisted.config.install_dir).exists()
        {
            persisted = store::load(&persisted.config.install_dir);
        }
        if let Err(err) = store::ensure_layout(&persisted.config.install_dir) {
            eprintln!("ultima: não foi possível preparar {}: {err}", persisted.config.install_dir.display());
        }

        let (tx, rx) = channel();
        let installs = persisted
            .installs
            .into_iter()
            .map(|record| (record.repo.clone(), record))
            .collect();

        let selected = persisted.config.repos.first().map(|r| r.full_name.clone());
        let dir_draft = persisted.config.install_dir.display().to_string();

        let mut app = Self {
            root: persisted.config.install_dir.clone(),
            config: persisted.config,
            installs,
            infos: HashMap::new(),
            releases: HashMap::new(),
            heads: HashMap::new(),
            errors: HashMap::new(),
            loading: HashSet::new(),
            selected,
            page: Page::Library,
            filter: Filter::All,
            search: String::new(),
            show_settings: false,
            token_draft: String::new(),
            dir_draft,
            new_repo_draft: String::new(),
            settings_error: None,
            jobs: Vec::new(),
            next_job: 1,
            toasts: Vec::new(),
            discover: None,
            discover_loading: false,
            discover_error: None,
            chosen: HashMap::new(),
            last_check: None,
            tx,
            rx,
        };
        app.token_draft = app.config.token.clone().unwrap_or_default();
        app.refresh_all();
        app
    }

    // -----------------------------------------------------------------------
    // Consultas
    // -----------------------------------------------------------------------

    pub fn repo(&self, id: &str) -> Option<&RepoRef> {
        self.config.repos.iter().find(|r| r.full_name == id)
    }

    pub fn latest_stable(&self, id: &str) -> Option<&Release> {
        self.releases
            .get(id)
            .and_then(|list| list.iter().filter(|r| !r.prerelease).max_by(|a, b| {
                crate::model::compare_versions(&a.tag, &b.tag)
            }))
    }

    pub fn has_updates(&self, id: &str) -> bool {
        let Some(installed) = self.installs.get(id) else {
            return false;
        };
        if installed.channel != Channel::Release {
            return false;
        }
        self.latest_stable(id)
            .map(|release| is_newer(&release.tag, &installed.version))
            .unwrap_or(false)
    }

    pub fn repos_source_only(&self) -> usize {
        self.config
            .repos
            .iter()
            .filter(|r| {
                self.releases
                    .get(&r.full_name)
                    .map(|list| list.is_empty())
                    .unwrap_or(false)
            })
            .count()
    }

    pub fn updates_count(&self) -> usize {
        self.config
            .repos
            .iter()
            .filter(|r| self.has_updates(&r.full_name))
            .count()
    }

    pub fn active_jobs(&self) -> usize {
        self.jobs.iter().filter(|j| !j.is_done()).count()
    }

    pub fn is_busy(&self, id: &str) -> bool {
        self.jobs
            .iter()
            .any(|j| j.repo == id && !j.is_done())
    }

    pub fn job_for(&self, id: &str) -> Option<&JobView> {
        self.jobs.iter().rev().find(|j| j.repo == id)
    }

    pub fn active_job_for(&self, id: &str) -> Option<&JobView> {
        self.jobs
            .iter()
            .rev()
            .find(|j| j.repo == id && !j.is_done())
    }

    pub fn chosen_version(&self, id: &str) -> String {
        if let Some(tag) = self.chosen.get(id) {
            return tag.clone();
        }
        self.latest_stable(id)
            .map(|r| r.tag.clone())
            .unwrap_or_else(|| self.branch_of(id))
    }

    pub fn branch_of(&self, id: &str) -> String {
        self.infos
            .get(id)
            .map(|i| i.default_branch.clone())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "main".into())
    }

    pub fn filtered_repos(&self) -> Vec<String> {
        let needle = self.search.trim().to_lowercase();
        self.config
            .repos
            .iter()
            .filter(|repo| {
                if needle.is_empty() {
                    return true;
                }
                let hay = format!(
                    "{} {} {} {}",
                    repo.full_name,
                    repo.title(),
                    repo.note,
                    self.infos
                        .get(&repo.full_name)
                        .and_then(|i| i.description.clone())
                        .unwrap_or_default()
                )
                .to_lowercase();
                hay.contains(&needle)
            })
            .filter(|repo| match self.page {
                Page::Updates => self.has_updates(&repo.full_name),
                Page::Library | Page::Discover => match self.filter {
                    Filter::All => true,
                    Filter::Installed => self.installs.contains_key(&repo.full_name),
                    Filter::Updates => self.has_updates(&repo.full_name),
                    Filter::SourceOnly => self
                        .releases
                        .get(&repo.full_name)
                        .map(|r| r.is_empty())
                        .unwrap_or(false),
                },
                Page::Builds => true,
            })
            .map(|repo| repo.full_name.clone())
            .collect()
    }

    pub fn subtitle_for(&self, id: &str) -> String {
        if let Some(repo) = self.repo(id) {
            if !repo.note.is_empty() {
                return repo.note.clone();
            }
        }
        self.infos
            .get(id)
            .and_then(|i| i.description.clone())
            .unwrap_or_else(|| "Repositório conectado".into())
    }

    pub fn family_of(&self, id: &str) -> String {
        self.repo(id)
            .map(|r| r.family.clone())
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| "Conectados".into())
    }

    pub fn families(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = Vec::new();
        for repo in &self.config.repos {
            let family = if repo.family.is_empty() { "Conectados" } else { &repo.family };
            match out.iter_mut().find(|(name, _)| name == family) {
                Some((_, count)) => *count += 1,
                None => out.push((family.to_string(), 1)),
            }
        }
        out
    }

    // -----------------------------------------------------------------------
    // Ações
    // -----------------------------------------------------------------------

    pub fn select_repo(&mut self, id: &str) {
        self.selected = Some(id.to_string());
    }

    pub fn goto(&mut self, page: Page) {
        self.page = page;
        if page == Page::Discover {
            self.load_discover();
        }
    }

    pub fn refresh_all(&mut self) {
        self.last_check = Some(Instant::now());
        let ids: Vec<String> = self.config.repos.iter().map(|r| r.full_name.clone()).collect();
        for id in ids {
            self.refresh_repo(&id);
        }
    }

    pub fn refresh_repo(&mut self, id: &str) {
        let Some(repo) = self.repo(id).cloned() else {
            return;
        };
        if !self.loading.insert(id.to_string()) {
            return;
        }
        worker::spawn_repo_refresh(self.tx.clone(), self.config.token.clone(), repo);
    }

    pub fn load_discover(&mut self) {
        if self.discover_loading || self.discover.is_some() {
            return;
        }
        let login = self
            .config
            .repos
            .first()
            .map(|r| r.owner().to_string())
            .unwrap_or_else(|| "wyvern9565656".into());
        self.discover_loading = true;
        self.discover_error = None;
        worker::spawn_discover(self.tx.clone(), self.config.token.clone(), login);
    }

    pub fn start_install(&mut self, id: &str) {
        if self.is_busy(id) {
            return;
        }
        let Some(repo) = self.repo(id).cloned() else {
            return;
        };
        let chosen = self.chosen_version(id);
        let Some(release) = self
            .releases
            .get(id)
            .and_then(|list| list.iter().find(|r| r.tag == chosen))
            .cloned()
        else {
            self.toast("Escolha uma versão publicada para instalar", LogLevel::Warn);
            return;
        };
        let Some(asset) = pick_asset(&release).cloned() else {
            self.toast(
                "O release não tem artefato compatível com Linux — use Compilar do código",
                LogLevel::Warn,
            );
            return;
        };
        let checksum_url = find_checksum_asset(&release, &asset).map(|a| a.url.clone());
        let label = format!("{} · {}", repo.title(), display_tag(&release.tag));

        let job = self.begin_job(
            id,
            label,
            format!("baixar {} + sha256 + extrair", asset.name),
        );
        worker::spawn_install(
            self.tx.clone(),
            self.config.token.clone(),
            self.root.clone(),
            InstallRequest {
                job,
                repo,
                release,
                asset,
                checksum_url,
            },
        );
    }

    pub fn start_update(&mut self, id: &str) {
        if let Some(tag) = self.latest_stable(id).map(|r| r.tag.clone()) {
            self.chosen.insert(id.to_string(), tag);
        }
        self.start_install(id);
    }

    pub fn start_compile(&mut self, id: &str) {
        if self.is_busy(id) {
            return;
        }
        let Some(repo) = self.repo(id).cloned() else {
            return;
        };
        let branch = self.branch_of(id);
        let label = format!("{} · {}", repo.title(), branch);
        let language = self.infos.get(id).and_then(|i| i.language.clone());
        let command = match language.as_deref() {
            Some("Rust") => "cargo build --release",
            Some("Python") => "empacotar python",
            Some("JavaScript") | Some("TypeScript") => "npm install && npm run build",
            _ => "detectar sistema de build",
        };
        let job = self.begin_job(id, label, command.to_string());
        worker::spawn_compile(
            self.tx.clone(),
            self.config.token.clone(),
            self.root.clone(),
            job,
            repo,
            branch,
        );
    }

    fn begin_job(&mut self, repo: &str, title: String, command: String) -> u64 {
        let id = self.next_job;
        self.next_job += 1;
        self.jobs.push(JobView {
            id,
            repo: repo.to_string(),
            title,
            command,
            log: Vec::new(),
            phase: "preparando".into(),
            frac: None,
            started: Instant::now(),
            done: None,
            record: None,
        });
        id
    }

    pub fn open_installed(&mut self, id: &str) {
        let Some(record) = self.installs.get(id) else {
            self.toast("Nada instalado neste repositório ainda", LogLevel::Warn);
            return;
        };
        let Some(binary) = record.binary.clone() else {
            self.toast(
                "Esta instalação não tem executável — abra a pasta de instalação",
                LogLevel::Warn,
            );
            return;
        };
        let mut command = std::process::Command::new(&binary);
        command.current_dir(&record.path);
        match command.spawn() {
            Ok(_) => self.toast(
                &format!("Abrindo {}", binary.file_name().unwrap_or_default().to_string_lossy()),
                LogLevel::Info,
            ),
            Err(err) => self.toast(&format!("Não foi possível abrir: {err}"), LogLevel::Error),
        }
    }

    pub fn toast(&mut self, text: &str, level: LogLevel) {
        self.toasts.push(Toast {
            text: text.to_string(),
            level,
            created: Instant::now(),
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn add_repo(&mut self, input: &str) {
        let mut value = input.trim().to_string();
        value = value
            .trim_start_matches("https://github.com/")
            .trim_start_matches("http://github.com/")
            .trim_start_matches("git@github.com:")
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string();
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            self.settings_error = Some("Use o formato dono/repositório".into());
            return;
        }
        let full = format!("{}/{}", parts[0], parts[1]);
        if self.config.repos.iter().any(|r| r.full_name.eq_ignore_ascii_case(&full)) {
            self.settings_error = Some("Este repositório já está conectado".into());
            return;
        }
        self.config.repos.push(RepoRef::new(
            &full,
            &crate::model::pretty_name(parts[1]),
            "Conectados",
            "",
        ));
        self.settings_error = None;
        self.new_repo_draft.clear();
        self.selected = Some(full.clone());
        self.save();
        self.refresh_repo(&full);
        self.toast(&format!("{full} conectado"), LogLevel::Info);
    }

    pub fn remove_repo(&mut self, id: &str) {
        self.config.repos.retain(|r| r.full_name != id);
        self.releases.remove(id);
        self.infos.remove(id);
        self.heads.remove(id);
        self.chosen.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = self.config.repos.first().map(|r| r.full_name.clone());
        }
        self.save();
    }

    pub fn save(&mut self) {
        let records: Vec<InstalledRecord> = self.installs.values().cloned().collect();
        if let Err(err) = store::save(&self.config.install_dir, &self.config, &records) {
            self.toast(&format!("Não foi possível salvar o estado: {err}"), LogLevel::Error);
        }
    }

    pub fn apply_settings(&mut self) {
        let token = self.token_draft.trim();
        self.config.token = if token.is_empty() { None } else { Some(token.to_string()) };

        let dir = PathBuf::from(self.dir_draft.trim());
        if self.dir_draft.trim().is_empty() {
            self.settings_error = Some("A pasta de instalação não pode ficar vazia".into());
            return;
        }
        if dir != self.config.install_dir {
            self.config.install_dir = dir;
            self.root = self.config.install_dir.clone();
            self.dir_draft = self.root.display().to_string();
            if let Err(err) = store::ensure_layout(&self.root) {
                self.settings_error = Some(format!("Não foi possível criar a pasta: {err}"));
                return;
            }
        }
        self.settings_error = None;
        self.save();
        self.show_settings = false;
        self.toast("Configurações salvas", LogLevel::Info);
        self.refresh_all();
    }

    pub fn install_dir_label(&self) -> String {
        crate::model::home_relative(&self.config.install_dir)
    }

    // -----------------------------------------------------------------------
    // Eventos
    // -----------------------------------------------------------------------

    fn pump(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Repo { repo, result } => {
                    self.loading.remove(&repo);
                    match result {
                        Ok(info) => {
                            self.errors.remove(&repo);
                            self.infos.insert(repo, info);
                        }
                        Err(err) => {
                            self.errors.insert(repo, err);
                        }
                    }
                }
                Event::Releases { repo, result } => {
                    match result {
                        Ok(releases) => {
                            self.releases.insert(repo.clone(), releases);
                            if let (Some(tag), true) = (
                                self.latest_stable(&repo).map(|r| r.tag.clone()),
                                !self.chosen.contains_key(&repo),
                            ) {
                                self.chosen.insert(repo, tag);
                            }
                        }
                        Err(err) => {
                            self.errors.insert(repo, err);
                        }
                    }
                }
                Event::Head { repo, result } => {
                    if let Ok((sha, message)) = result {
                        self.heads.insert(repo, (sha, message));
                    }
                }
                Event::Discover { result } => {
                    self.discover_loading = false;
                    match result {
                        Ok(repos) => self.discover = Some(repos),
                        Err(err) => self.discover_error = Some(err),
                    }
                }
                Event::Log { job, level, line } => {
                    if let Some(view) = self.jobs.iter_mut().find(|j| j.id == job) {
                        view.log.push((level, line));
                        if view.log.len() > 4000 {
                            view.log.drain(0..1000);
                        }
                    }
                }
                Event::Phase { job, phase, frac } => {
                    if let Some(view) = self.jobs.iter_mut().find(|j| j.id == job) {
                        view.phase = phase;
                        view.frac = frac;
                    }
                }
                Event::Done { job, record, error } => {
                    let repo = self.jobs.iter().find(|j| j.id == job).map(|j| j.repo.clone());
                    if let Some(view) = self.jobs.iter_mut().find(|j| j.id == job) {
                        view.done = Some(match &error {
                            Some(err) => Err(err.clone()),
                            None => Ok(()),
                        });
                        view.record = record.clone();
                    }
                    match error {
                        None => {
                            if let (Some(record), Some(repo)) = (record.clone(), repo.clone()) {
                                self.installs.insert(repo.clone(), record.clone());
                                self.save();
                                self.chosen.insert(repo, record.version.clone());
                                self.toast(
                                    &format!("{} instalado ({})", record.repo, record.label()),
                                    LogLevel::Info,
                                );
                            }
                        }
                        Some(err) => {
                            self.toast(&format!("Falhou: {err}"), LogLevel::Error);
                        }
                    }
                }
            }
        }

        self.toasts.retain(|t| t.created.elapsed() < Duration::from_secs(6));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.pump();

        ui::sidebar::show(self, ui);
        ui::details::show(self, ui);
        ui::central(self, ui);

        ui::settings::show(self, &ctx);
        ui::toasts(self, &ctx);

        // Mantém a UI viva enquanto há requisições, jobs ou toasts em andamento.
        if self.active_jobs() > 0
            || !self.loading.is_empty()
            || self.discover_loading
            || !self.toasts.is_empty()
        {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtro_de_busca_e_familia() {
        let config = Config::default();
        assert_eq!(config.repos.len(), 4);
        let app = App {
            root: PathBuf::from("/tmp/ultima-test"),
            config,
            installs: HashMap::new(),
            infos: HashMap::new(),
            releases: HashMap::new(),
            heads: HashMap::new(),
            errors: HashMap::new(),
            loading: HashSet::new(),
            selected: None,
            page: Page::Library,
            filter: Filter::All,
            search: "pulse".into(),
            show_settings: false,
            token_draft: String::new(),
            dir_draft: String::new(),
            new_repo_draft: String::new(),
            settings_error: None,
            jobs: Vec::new(),
            next_job: 1,
            toasts: Vec::new(),
            discover: None,
            discover_loading: false,
            discover_error: None,
            chosen: HashMap::new(),
            last_check: None,
            tx: channel().0,
            rx: channel().1,
        };
        assert_eq!(app.filtered_repos(), vec!["wyvern9565656/ultima-pulse".to_string()]);
        assert_eq!(app.families().len(), 2);
    }
}
