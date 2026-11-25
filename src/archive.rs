use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

pub struct DownloadOutcome {
    pub sha256: String,
    pub bytes: u64,
}

/// Baixa `url` para `dest` calculando o SHA-256 enquanto escreve.
/// `progress` recebe (bytes recebidos, total conhecido ou 0).
pub fn download(
    url: &str,
    dest: &Path,
    token: Option<&str>,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<DownloadOutcome, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("não foi possível criar {}: {e}", parent.display()))?;
    }
    let client = reqwest::blocking::Client::builder()
        .user_agent("Ultima/0.1")
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("cliente HTTP: {e}"))?;

    let mut req = client.get(url).header("Accept", "application/octet-stream");
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        req = req.bearer_auth(token);
    }
    let mut resp = req.send().map_err(|e| format!("falha ao baixar {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download falhou (HTTP {}) para {url}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let mut file = File::create(dest).map_err(|e| format!("não foi possível salvar {}: {e}", dest.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut received: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    progress(0, total);
    loop {
        let read = resp
            .read(&mut buffer)
            .map_err(|e| format!("leitura interrompida: {e}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("falha de escrita em {}: {e}", dest.display()))?;
        hasher.update(&buffer[..read]);
        received += read as u64;
        if last_emit.elapsed() > Duration::from_millis(120) {
            progress(received, total);
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().map_err(|e| format!("falha ao finalizar o arquivo: {e}"))?;
    progress(received, total.max(received));

    Ok(DownloadOutcome {
        sha256: format!("{:x}", hasher.finalize()),
        bytes: received,
    })
}

/// Baixa um arquivo de texto pequeno (ex.: lista de checksums).
pub fn fetch_text(url: &str, token: Option<&str>) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Ultima/0.1")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("cliente HTTP: {e}"))?;
    let mut req = client.get(url);
    if let Some(token) = token.filter(|t| !t.is_empty()) {
        req = req.bearer_auth(token);
    }
    let resp = req.send().map_err(|e| format!("falha ao baixar {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} ao baixar {url}", resp.status()));
    }
    resp.text().map_err(|e| format!("ler {url}: {e}"))
}

/// Normaliza `sha256:<hex>` / uppercase para minúsculas sem prefixo.
pub fn normalize_sha(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("sha256:")
        .trim()
        .to_lowercase()
}

pub fn verify_sha256(expected: &str, actual: &str) -> Result<(), String> {
    let expected = normalize_sha(expected);
    let actual = normalize_sha(actual);
    if expected.len() != 64 {
        return Err(format!("checksum esperado inválido: {expected}"));
    }
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "checksum não confere: esperado {}…, obtido {}…",
            &expected[..12],
            &actual[..12]
        ))
    }
}

/// Extrai zip, tar, tar.gz/tgz e, como fallback, delega ao `tar` do sistema.
pub fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("não foi possível criar {}: {e}", dest.display()))?;
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if name.ends_with(".zip") {
        extract_zip(archive, dest)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar(archive, dest, true)
    } else if name.ends_with(".tar") {
        extract_tar(archive, dest, false)
    } else if name.ends_with(".appimage") {
        let target = dest.join(archive.file_name().unwrap_or_default());
        fs::copy(archive, &target).map_err(|e| format!("não foi possível copiar o AppImage: {e}"))?;
        make_executable(&target);
        Ok(())
    } else {
        // Formatos menos comuns (.tar.xz, .tar.zst, .deb): usa o tar do sistema.
        let status = std::process::Command::new("tar")
            .arg("-xf")
            .arg(archive)
            .arg("-C")
            .arg(dest)
            .status();
        match status {
            Ok(s) if s.success() => Ok(()),
            _ => Err(format!(
                "formato não suportado: {} (use .zip, .tar.gz ou .AppImage)",
                archive.display()
            )),
        }
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("abrir zip: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip inválido: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("entrada {i} do zip: {e}"))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err("zip contém caminho inseguro".into());
        };
        let out_path = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| format!("criar pasta no zip: {e}"))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("criar pasta no zip: {e}"))?;
        }
        let mut out = File::create(&out_path).map_err(|e| format!("extrair {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("extrair {}: {e}", out_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
            }
        }
    }
    Ok(())
}

fn extract_tar(archive: &Path, dest: &Path, gzip: bool) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("abrir tar: {e}"))?;
    if gzip {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        tar.set_preserve_permissions(true);
        tar.unpack(dest).map_err(|e| format!("extrair tar.gz: {e}"))
    } else {
        let mut tar = tar::Archive::new(file);
        tar.set_preserve_permissions(true);
        tar.unpack(dest).map_err(|e| format!("extrair tar: {e}"))
    }
}

#[cfg(unix)]
pub fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        let mode = perms.mode();
        perms.set_mode(mode | 0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) {}

fn looks_like_binary(path: &Path, executable: bool) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let lower = name.to_lowercase();
    if lower.is_empty()
        || lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".dylib")
        || lower.ends_with(".a")
        || lower.ends_with(".rlib")
        || lower.ends_with(".txt")
        || lower.ends_with(".md")
        || lower.ends_with(".json")
        || lower.ends_with(".toml")
        || lower.ends_with(".png")
        || lower.ends_with(".svg")
        || lower.ends_with(".jpg")
        || lower.ends_with(".desktop")
    {
        return false;
    }
    if executable {
        return true;
    }
    // Alguns zips não preservam o bit de execução.
    !name.contains('.') && fs::metadata(path).map(|m| m.len() > 8 * 1024).unwrap_or(false)
}

/// Procura o executável instalado, priorizando nomes conhecidos.
pub fn find_binary(root: &Path, hints: &[&str], max_depth: usize) -> Option<PathBuf> {
    let mut best: Option<(i32, u64, PathBuf)> = None;
    let walker = walkdir::WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "target" || e.depth() == 0
        });
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let executable = is_executable(path);
        if !looks_like_binary(path, executable) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        let mut score = 0;
        if hints.iter().any(|h| h.to_lowercase() == name) {
            score += 100;
        }
        if executable {
            score += 10;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if score > best.as_ref().map(|(s, _, _)| *s).unwrap_or(-1)
            || (best.as_ref().map(|(s, sz, _)| (*s, *sz)) < Some((score, size)))
        {
            best = Some((score, size, path.to_path_buf()));
        }
    }
    best.map(|(_, _, p)| p)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

/// Remove duplicatas de diretório raiz única (`app-v1/...` -> `...`).
pub fn flatten_single_root(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let items: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    if items.len() != 1 || !items[0].is_dir() {
        return;
    }
    let inner = &items[0];
    let Ok(children) = fs::read_dir(inner) else { return };
    let children: Vec<PathBuf> = children.flatten().map(|e| e.path()).collect();
    for child in children {
        let target = dir.join(child.file_name().unwrap_or_default());
        if fs::rename(&child, &target).is_ok() {
            continue;
        }
        // Renomear pode falhar entre sistemas de arquivos; copia como fallback.
        let _ = copy_recursive(&child, &target);
    }
    let _ = fs::remove_dir_all(inner);
}

fn copy_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extrai_zip_com_binario() {
        let root = std::env::temp_dir().join(format!("ultima-zip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("app.zip");
        {
            let file = File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();
            zip.start_file("app-linux/nexus", options).unwrap();
            zip.write_all(&vec![7u8; 16 * 1024]).unwrap();
            zip.start_file("app-linux/LEIAME.md", options).unwrap();
            zip.write_all(b"oi").unwrap();
            zip.finish().unwrap();
        }
        let dest = root.join("out");
        extract(&archive, &dest).unwrap();
        flatten_single_root(&dest);
        assert!(dest.join("nexus").is_file());
        assert!(!dest.join("app-linux").exists());
        make_executable(&dest.join("nexus"));
        let found = find_binary(&dest, &["nexus"], 4).unwrap();
        assert!(found.ends_with("nexus"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn extrai_tar_gz() {
        let root = std::env::temp_dir().join(format!("ultima-targz-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("app.tar.gz");
        {
            let file = File::create(&archive).unwrap();
            let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
            let mut tar = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(16 * 1024);
            header.set_mode(0o755);
            header.set_cksum();
            let data = vec![1u8; 16 * 1024];
            tar.append_data(&mut header, "pulse/ultima-pulse", data.as_slice())
                .unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        let dest = root.join("out");
        extract(&archive, &dest).unwrap();
        flatten_single_root(&dest);
        assert!(dest.join("ultima-pulse").is_file());
        let found = find_binary(&dest, &["ultima-pulse"], 4).unwrap();
        assert!(found.ends_with("ultima-pulse"));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn valida_sha() {
        let hash = "a".repeat(64);
        assert!(verify_sha256(&hash, &hash.to_uppercase()).is_ok());
        assert!(verify_sha256(&format!("sha256:{hash}"), &hash).is_ok());
        assert!(verify_sha256(&hash, &"b".repeat(64)).is_err());
        assert!(verify_sha256("curto", &hash).is_err());
    }

    #[test]
    fn encontra_binario_por_nome() {
        let root = std::env::temp_dir().join(format!("ultima-bin-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("LEIAME.md"), "oi").unwrap();
        fs::write(root.join("sub").join("nexus"), vec![0u8; 16 * 1024]).unwrap();
        make_executable(&root.join("sub").join("nexus"));
        let found = find_binary(&root, &["nexus"], 5).unwrap();
        assert!(found.ends_with("sub/nexus"));
        fs::remove_dir_all(&root).ok();
    }
}

