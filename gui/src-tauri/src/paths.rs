use std::path::PathBuf;

/// Dossier portable : là où vit l'exe (+ settings.json, bin/, *.json).
/// En dev (target/debug), remonte jusqu'à trouver settings.json.
pub fn spoofer_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("exe introuvable: {e}"))?;
    let start = exe.parent().map(|p| p.to_path_buf()).ok_or("pas de dossier exe")?;
    let mut dir = start.clone();
    for _ in 0..7 {
        if dir.join("settings.json").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    // repli : dossier de l'exe (les fichiers seront créés à côté)
    Ok(start)
}

pub fn settings_path() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("settings.json"))
}

pub fn originals_path() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("originals.json"))
}
