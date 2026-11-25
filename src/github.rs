use std::time::Duration;

use serde::Deserialize;

use crate::model::{Asset, Release, RepoInfo};

const API: &str = "https://api.github.com";

pub struct Github {
    client: reqwest::blocking::Client,
    token: Option<String>,
}

fn describe_status(status: reqwest::StatusCode, url: &str) -> String {
    match status.as_u16() {
        401 => "token do GitHub inválido ou expirado".to_string(),
        403 | 429 => format!(
            "limite de requisições do GitHub atingido ({url}). Configure um token em Configurações."
        ),
        404 => "não encontrado no GitHub (repositório privado ou inexistente)".to_string(),
        other => format!("GitHub respondeu {other} para {url}"),
    }
}

impl Github {
    pub fn new(token: Option<String>) -> Self {
        let token = token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let client = reqwest::blocking::Client::builder()
            .user_agent("Ultima/0.1 (+https://github.com/wyvern9565656)")
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self { client, token }
    }

    fn request(&self, url: &str) -> Result<reqwest::blocking::Response, String> {
        let mut req = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().map_err(|e| format!("falha de rede: {e}"))?;
        if !resp.status().is_success() {
            return Err(describe_status(resp.status(), url));
        }
        Ok(resp)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, String> {
        let resp = self.request(url)?;
        resp.json::<T>()
            .map_err(|e| format!("resposta inesperada do GitHub: {e}"))
    }

    pub fn fetch_repo(&self, full_name: &str) -> Result<RepoInfo, String> {
        let raw: ApiRepo = self.get_json(&format!("{API}/repos/{full_name}"))?;
        Ok(raw.into())
    }

    pub fn fetch_releases(&self, full_name: &str) -> Result<Vec<Release>, String> {
        let raw: Vec<ApiRelease> =
            self.get_json(&format!("{API}/repos/{full_name}/releases?per_page=30"))?;
        Ok(raw
            .into_iter()
            .filter(|r| !r.draft)
            .map(Release::from)
            .collect())
    }

    pub fn fetch_user_repos(&self, login: &str) -> Result<Vec<RepoInfo>, String> {
        let raw: Vec<ApiRepo> = self.get_json(&format!(
            "{API}/users/{login}/repos?per_page=100&sort=pushed"
        ))?;
        Ok(raw.into_iter().map(RepoInfo::from).collect())
    }

    pub fn head_commit(&self, full_name: &str, branch: &str) -> Result<(String, String), String> {
        let raw: Vec<ApiCommit> = self.get_json(&format!(
            "{API}/repos/{full_name}/commits?sha={branch}&per_page=1"
        ))?;
        let first = raw.into_iter().next().ok_or("repositório sem commits")?;
        let sha: String = first.sha.chars().take(7).collect();
        Ok((sha, first.commit.message.lines().next().unwrap_or("").to_string()))
    }
}

// ---------------------------------------------------------------------------
// Mapeamento da API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ApiRepo {
    full_name: String,
    description: Option<String>,
    language: Option<String>,
    default_branch: Option<String>,
    stargazers_count: Option<u64>,
    pushed_at: Option<String>,
    archived: Option<bool>,
    html_url: Option<String>,
}

impl From<ApiRepo> for RepoInfo {
    fn from(raw: ApiRepo) -> Self {
        Self {
            full_name: raw.full_name,
            description: raw.description,
            language: raw.language,
            default_branch: raw.default_branch.unwrap_or_else(|| "main".into()),
            stars: raw.stargazers_count.unwrap_or(0),
            pushed_at: raw.pushed_at,
            archived: raw.archived.unwrap_or(false),
            html_url: raw.html_url.unwrap_or_default(),
        }
    }
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    name: Option<String>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    published_at: Option<String>,
    body: Option<String>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

impl From<ApiRelease> for Release {
    fn from(raw: ApiRelease) -> Self {
        Self {
            tag: raw.tag_name,
            name: raw.name.unwrap_or_default(),
            prerelease: raw.prerelease,
            published_at: raw.published_at,
            notes: raw.body.unwrap_or_default(),
            assets: raw
                .assets
                .into_iter()
                .map(|a| Asset {
                    name: a.name,
                    size: a.size,
                    url: a.browser_download_url,
                    digest: a.digest,
                })
                .collect(),
        }
    }
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    #[serde(default)]
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct ApiCommit {
    sha: String,
    commit: ApiCommitInner,
}

#[derive(Deserialize)]
struct ApiCommitInner {
    message: String,
}

