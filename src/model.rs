use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Canal de instalação de um app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// Artefato publicado em um release do GitHub.
    Release,
    /// Compilado a partir do código-fonte (branch padrão).
    Source,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoRef {
    pub full_name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub note: String,
}

impl RepoRef {
    pub fn new(full_name: &str, title: &str, family: &str, note: &str) -> Self {
        Self {
            full_name: full_name.to_string(),
            title: title.to_string(),
            family: family.to_string(),
            note: note.to_string(),
        }
    }

    pub fn owner(&self) -> &str {
        self.full_name.split('/').next().unwrap_or("")
    }

    pub fn name(&self) -> &str {
        self.full_name.split('/').nth(1).unwrap_or(&self.full_name)
    }

    /// Identificador estável e seguro para usar como nome de pasta.
    pub fn slug(&self) -> String {
        self.full_name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
            .collect()
    }

    pub fn title(&self) -> String {
        if !self.title.is_empty() {
            self.title.clone()
        } else {
            pretty_name(self.name())
        }
    }

    pub fn clone_url(&self) -> String {
        format!("https://github.com/{}.git", self.full_name)
    }

}

pub fn pretty_name(name: &str) -> String {
    let mut out = String::new();
    for (i, part) in name.split(['-', '_']).filter(|p| !p.is_empty()).enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepoInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub default_branch: String,
    pub stars: u64,
    pub pushed_at: Option<String>,
    pub archived: bool,
    pub html_url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub url: String,
    /// Digest informado pela API do GitHub, no formato `sha256:<hex>`.
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub notes: String,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstalledRecord {
    pub repo: String,
    pub version: String,
    pub channel: Channel,
    pub path: PathBuf,
    pub binary: Option<PathBuf>,
    pub installed_at: u64,
    pub checksum: Option<String>,
}

impl InstalledRecord {
    /// Rótulo exibido para a versão instalada (tags levam `v`, branches não).
    pub fn label(&self) -> String {
        match self.channel {
            Channel::Release => display_tag(&self.version),
            Channel::Source => self.version.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub install_dir: PathBuf,
    #[serde(default)]
    pub token: Option<String>,
    pub repos: Vec<RepoRef>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            install_dir: default_install_dir(),
            token: None,
            repos: vec![
                RepoRef::new("wyvern9565656/ultima-pulse", "Ultima Pulse", "Ultima", "Monitor de sistema"),
                RepoRef::new("wyvern9565656/Nexus", "Nexus", "Nexus", "Gerenciador de arquivos"),
                RepoRef::new("wyvern9565656/ultima-launcher", "UltimaLauncher", "Ultima", "Launcher de jogos"),
                RepoRef::new("wyvern9565656/nexus-code", "Nexus Code", "Nexus", "Editor de código"),
            ],
        }
    }
}

pub fn default_install_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ultima")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Persisted {
    #[serde(default)]
    pub config: Config,
    #[serde(default)]
    pub installs: Vec<InstalledRecord>,
}

// ---------------------------------------------------------------------------
// Versões
// ---------------------------------------------------------------------------

/// Extrai a parte numérica de uma tag (`v0.3` -> `[0, 3]`) e sinaliza pré-release.
pub fn parse_version(tag: &str) -> (Vec<u64>, bool) {
    let t = tag.trim().trim_start_matches(['v', 'V']);
    let (numeric, pre) = match t.split_once('-') {
        Some((n, p)) => (n, !p.is_empty()),
        None => (t, false),
    };
    let nums = numeric
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect();
    (nums, pre)
}

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (av, apre) = parse_version(a);
    let (bv, bpre) = parse_version(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    // Versão final é mais nova que a mesma versão em pré-release.
    match (apre, bpre) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        _ => Ordering::Equal,
    }
}

pub fn is_newer(candidate: &str, than: &str) -> bool {
    compare_versions(candidate, than) == Ordering::Greater
}

/// Tag exibida com prefixo `v`, sem duplicar quando já existe.
pub fn display_tag(tag: &str) -> String {
    if tag.starts_with('v') || tag.starts_with('V') {
        tag.to_string()
    } else {
        format!("v{tag}")
    }
}

// ---------------------------------------------------------------------------
// Escolha de artefato para a plataforma atual
// ---------------------------------------------------------------------------

fn asset_is_checksum(name: &str) -> bool {
    let n = name.to_lowercase();
    n.ends_with(".sha256")
        || n.ends_with(".sha256sum")
        || n.ends_with(".sha512")
        || n.ends_with(".sig")
        || n.ends_with(".asc")
        || n.ends_with("checksums.txt")
        || n.ends_with("checksum.txt")
        || n.contains("sha256sum")
}

fn asset_is_other_platform(name: &str) -> bool {
    let n = name.to_lowercase();
    let foreign = [
        "windows", "win32", "win64", "win-", ".exe", ".msi", ".dmg", ".pkg", "macos", "darwin",
        "apple", "aarch64-apple", "universal2",
    ];
    foreign.iter().any(|k| n.contains(k))
}

fn asset_score(asset: &Asset) -> i32 {
    if asset_is_checksum(&asset.name) {
        return -1;
    }
    let n = asset.name.to_lowercase();
    let mut score = 0;
    if asset_is_other_platform(&n) {
        return -1;
    }
    if n.contains("linux") {
        score += 20;
    }
    if n.contains(std::env::consts::ARCH) {
        score += 6;
    }
    if n.contains("x86_64") || n.contains("amd64") {
        score += 4;
    }
    if n.ends_with(".appimage") {
        score += 12;
    } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
        score += 10;
    } else if n.ends_with(".zip") {
        score += 9;
    } else if n.ends_with(".tar") || n.ends_with(".tar.zst") || n.ends_with(".tar.xz") {
        score += 7;
    } else {
        score += 3;
    }
    if n.contains("debug") || n.contains("symbols") {
        score -= 8;
    }
    score
}

/// Artefato mais adequado para esta máquina, se houver.
pub fn pick_asset(release: &Release) -> Option<&Asset> {
    release
        .assets
        .iter()
        .map(|a| (asset_score(a), a))
        .filter(|(score, _)| *score > 0)
        .max_by_key(|(score, a)| (*score, a.size.min(1_000_000)))
        .map(|(_, a)| a)
}

/// Arquivo de checksums publicado junto do artefato, se existir.
pub fn find_checksum_asset<'a>(release: &'a Release, target: &Asset) -> Option<&'a Asset> {
    release
        .assets
        .iter()
        .filter(|a| asset_is_checksum(&a.name))
        .find(|a| {
            let n = a.name.to_lowercase();
            n.contains(&target.name.to_lowercase())
                || n.contains("checksum")
                || n.contains("sha256")
        })
}

/// Procura o hash do arquivo em um arquivo de checksums (`hash  nome` ou `SHA256 (nome) = hash`).
pub fn parse_checksum_file(contents: &str, filename: &str) -> Option<String> {
    let target = filename.to_lowercase();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == '*' || c == '(' || c == ')' || c == '=')
            .filter(|t| !t.is_empty())
            .collect();
        let hash = tokens.iter().find(|t| {
            t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit())
        });
        let (Some(hash), Some(name)) = (hash, tokens.last()) else {
            continue;
        };
        if name.to_lowercase().ends_with(&target) || target.ends_with(&name.to_lowercase()) {
            return Some(hash.to_lowercase());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Utilidades de formatação
// ---------------------------------------------------------------------------

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn human_ago(epoch: u64) -> String {
    if epoch == 0 {
        return "nunca".into();
    }
    let now = now_epoch().saturating_sub(epoch);
    match now {
        0..=59 => "agora".into(),
        60..=3599 => format!("há {} min", now / 60),
        3600..=86_399 => format!("há {} h", now / 3600),
        86_400..=2_591_999 => format!("há {} d", now / 86_400),
        _ => {
            let (y, m, d) = civil_from_days((epoch / 86_400) as i64);
            format!("{d:02}/{m:02}/{y}")
        }
    }
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Formata `2026-11-25T01:37:16Z` como `25/11/2026`.
pub fn short_date(iso: &str) -> String {
    if iso.len() < 10 {
        return iso.to_string();
    }
    let date = &iso[..10];
    let mut parts = date.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d)) => format!("{d}/{m}/{y}"),
        _ => date.to_string(),
    }
}

pub fn home_relative(path: &std::path::Path) -> String {
    let mut shown = path.display().to_string();
    if let Some(home) = dirs::home_dir() {
        if let Some(rest) = shown.strip_prefix(&home.display().to_string()) {
            shown = format!("~{rest}");
        }
    }
    if !shown.ends_with('/') {
        shown.push('/');
    }
    shown
}

/// Cor de destaque associada ao repositório (estável entre execuções).
pub fn accent_for(repo: &str) -> egui::Color32 {
    let name = repo.rsplit('/').next().unwrap_or(repo).to_lowercase();
    if name.contains("pulse") {
        egui::Color32::from_rgb(0x3D, 0xE0, 0xBF)
    } else if name == "nexus" {
        egui::Color32::from_rgb(0xA7, 0x8B, 0xFF)
    } else if name.contains("launcher") {
        egui::Color32::from_rgb(0xF0, 0x8B, 0xBD)
    } else if name.contains("code") {
        egui::Color32::from_rgb(0xF5, 0xA0, 0x4A)
    } else {
        let hash = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        let palette = [
            egui::Color32::from_rgb(0x7F, 0xB8, 0xF5),
            egui::Color32::from_rgb(0x9B, 0xE0, 0x8B),
            egui::Color32::from_rgb(0xE8, 0x9B, 0x7F),
            egui::Color32::from_rgb(0xB9, 0x9B, 0xE8),
        ];
        palette[(hash as usize) % palette.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versoes_numericas() {
        assert_eq!(compare_versions("v0.3", "v0.2"), Ordering::Greater);
        assert_eq!(compare_versions("v1.10", "v1.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.3", "v0.3"), Ordering::Equal);
        assert_eq!(compare_versions("v2", "v2.0.1"), Ordering::Less);
        assert!(is_newer("v0.4", "v0.3"));
        assert!(!is_newer("v0.3", "v0.3"));
    }

    #[test]
    fn pre_release_e_menor() {
        assert_eq!(compare_versions("v0.3-beta", "v0.3"), Ordering::Less);
        assert_eq!(compare_versions("v0.4-beta", "v0.3"), Ordering::Greater);
    }

    #[test]
    fn escolhe_asset_linux() {
        let release = Release {
            tag: "v1.0".into(),
            assets: vec![
                Asset { name: "app-windows.exe".into(), size: 10, url: String::new(), digest: None },
                Asset { name: "app-x86_64-unknown-linux-gnu.tar.gz".into(), size: 20, url: "u".into(), digest: None },
                Asset { name: "app_1.0_amd64.deb".into(), size: 15, url: "d".into(), digest: None },
                Asset { name: "checksums.txt".into(), size: 1, url: "c".into(), digest: None },
            ],
            ..Default::default()
        };
        let picked = pick_asset(&release).expect("asset linux");
        assert_eq!(picked.name, "app-x86_64-unknown-linux-gnu.tar.gz");
        assert!(find_checksum_asset(&release, picked).is_some());
    }

    #[test]
    fn le_arquivo_de_checksum() {
        let hash = "a".repeat(64);
        let text = format!("{hash}  app-linux.tar.gz\n{}\tapp.zip", "b".repeat(64));
        assert_eq!(parse_checksum_file(&text, "app-linux.tar.gz"), Some(hash.clone()));
        assert_eq!(parse_checksum_file(&text, "app.zip"), Some("b".repeat(64)));
        assert_eq!(parse_checksum_file(&text, "outro.tar.gz"), None);
    }

    #[test]
    fn caminho_home_relativo() {
        let home = dirs::home_dir().unwrap();
        assert!(home_relative(&home.join("ultima")).starts_with("~/"));
    }
}
