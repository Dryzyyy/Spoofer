use std::fs::File;
use std::path::Path;
use std::time::Duration;

/// Télécharge url -> dest. log(texte) pour la progression.
pub fn download(
    url: &str,
    dest: &Path,
    log: &dyn Fn(&str),
    label: &str,
    timeout_secs: u64,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("GhostNet/1.0")
        .build()
        .map_err(|e| format!("client http: {e}"))?;
    log(&format!("[DL] {label}…"));
    let resp = client.get(url).send().map_err(|e| format!("téléchargement {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("téléchargement {url}: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| format!("lecture réseau: {e}"))?;
    log(&format!("[DL] {label} : {} Mo reçus, écriture…", bytes.len() / 1_000_000));
    use std::io::Write;
    let mut file = File::create(dest).map_err(|e| format!("création fichier: {e}"))?;
    file.write_all(&bytes).map_err(|e| format!("écriture: {e}"))?;
    Ok(())
}
