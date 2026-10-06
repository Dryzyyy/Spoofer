use serde_json::json;
use std::fs::File;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::fetch::download;
use crate::paths::spoofer_dir;
use crate::ps::{kill_pid, pid_alive};

pub const TOR_URL: &str = "https://dist.torproject.org/torbrowser/15.0.24/tor-expert-bundle-windows-x86_64-15.0.24.tar.gz";
pub const SOCKS_HOST: &str = "127.0.0.1";
pub const SOCKS_PORT: u16 = 9050;

fn tor_dir() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("bin").join("tor"))
}
fn data_dir() -> Result<PathBuf, String> {
    Ok(tor_dir()?.join("data"))
}
fn torrc() -> Result<PathBuf, String> {
    Ok(tor_dir()?.join("torrc"))
}
fn tor_log() -> Result<PathBuf, String> {
    Ok(tor_dir()?.join("tor.log"))
}
fn pid_file() -> Result<PathBuf, String> {
    Ok(tor_dir()?.join("tor.pid"))
}

fn hide(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
}

fn find_exe() -> Option<PathBuf> {
    let root = tor_dir().ok()?;
    let mut stack = vec![root];
    let mut depth = 0;
    while let Some(dir) = stack.pop() {
        depth += 1;
        if depth > 40 {
            break;
        }
        let entries = std::fs::read_dir(&dir).ok()?;
        for e in entries.filter_map(|r| r.ok()) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if e.file_name().to_string_lossy().eq_ignore_ascii_case("tor.exe") {
                return Some(p);
            }
        }
    }
    None
}

pub fn is_installed() -> bool {
    find_exe().is_some()
}

/// Télécharge + extrait le bundle officiel. log(texte) pour le journal.
pub fn install(log: &dyn Fn(&str)) -> Result<bool, String> {
    let dir = tor_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("bin/tor: {e}"))?;
    log("[Tor] Téléchargement Tor officiel…");
    let tmp = std::env::temp_dir().join("tor-expert.tar.gz");
    download(TOR_URL, &tmp, log, "Tor officiel", 600)?;
    log("[Tor] Extraction…");
    let f = File::open(&tmp).map_err(|e| format!("tar: {e}"))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);
    let entries = ar.entries().map_err(|e| format!("tar: {e}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("tar: {e}"))?;
        let path = entry.path().map_err(|e| format!("tar: {e}"))?.to_path_buf();
        // aplatit le dossier racine tor/ du tar
        let mut comps = path.components();
        comps.next();
        let rel: PathBuf = comps.as_path().to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest = dir.join(&rel);
        if entry.header().entry_type().is_dir() {
            let _ = std::fs::create_dir_all(&dest);
        } else {
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            entry.unpack(&dest).map_err(|e| format!("extraction: {e}"))?;
        }
    }
    let _ = std::fs::remove_file(&tmp);
    let ok = is_installed();
    log(if ok { "[Tor] prêt ✓" } else { "[Tor] échec install." });
    Ok(ok)
}

fn write_torrc() -> Result<(), String> {
    let exe = find_exe().ok_or("Tor non installé.")?;
    let exe_dir = exe.parent().ok_or("chemin tor.exe")?.to_path_buf();
    let data = data_dir()?;
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let logp = tor_log()?;
    let _ = File::create(&logp);
    let geo = exe_dir.join("geoip");
    let geo6 = exe_dir.join("geoip6");
    let data_s = data.to_string_lossy().replace('\\', "/");
    let log_s = logp.to_string_lossy().replace('\\', "/");
    let mut lines = vec![
        format!("SocksPort {SOCKS_HOST}:{SOCKS_PORT}"),
        format!("DataDirectory {data_s}"),
        "AvoidDiskWrites 1".into(),
        format!("Log notice file {log_s}"),
    ];
    if geo.exists() {
        lines.push(format!("GeoIPFile {}", geo.to_string_lossy().replace('\\', "/")));
    }
    if geo6.exists() {
        lines.push(format!("GeoIPv6File {}", geo6.to_string_lossy().replace('\\', "/")));
    }
    std::fs::write(torrc()?, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn is_running() -> bool {
    let p = match pid_file() {
        Ok(p) => p,
        Err(_) => return false,
    };
    match std::fs::read_to_string(&p) {
        Ok(t) => t.trim().parse::<u32>().map(pid_alive).unwrap_or(false),
        Err(_) => false,
    }
}

pub fn socks_ready() -> bool {
    // Timeout court : un port ouvert répond en ms ; un port fermé coûte
    // ~2s sur cette machine (SYN localhost sans RST immédiat) — inutile d'attendre plus.
    let addr = format!("{SOCKS_HOST}:{SOCKS_PORT}");
    let addrs: Vec<SocketAddr> = match addr.parse() {
        Ok(a) => vec![a],
        Err(_) => return false,
    };
    addrs
        .into_iter()
        .any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(1)).is_ok())
}

/// % de bootstrap lu dans tor.log (None si inconnu).
pub fn bootstrap_pct() -> Option<u32> {
    let p = tor_log().ok()?;
    let txt = std::fs::read_to_string(&p).ok()?;
    let tail: String = txt.chars().rev().take(6000).collect::<String>().chars().rev().collect();
    let mut last: Option<u32> = None;
    let mut search = tail.as_str();
    while let Some(i) = search.find("Bootstrapped ") {
        let rest = &search[i + "Bootstrapped ".len()..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num.parse::<u32>() {
            last = Some(n);
        }
        search = rest;
    }
    last
}

pub fn start() -> Result<u32, String> {
    if !is_installed() {
        return Err("Tor non installé.".into());
    }
    if is_running() {
        let t = std::fs::read_to_string(pid_file()?).unwrap_or_default();
        return t.trim().parse::<u32>().map_err(|e| e.to_string());
    }
    write_torrc()?;
    let exe = find_exe().ok_or("Tor non installé.")?;
    let rc = torrc()?;
    let logp = tor_log()?;
    let logf = File::options().create(true).append(true).open(&logp).map_err(|e| e.to_string())?;
    let logf2 = logf.try_clone().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&exe);
    cmd.args(["-f"]);
    cmd.arg(&rc);
    cmd.stdout(Stdio::from(logf));
    cmd.stderr(Stdio::from(logf2));
    hide(&mut cmd);
    let child = cmd.spawn().map_err(|e| format!("lancement tor: {e}"))?;
    let pid = child.id();
    std::mem::forget(child);
    let _ = std::fs::write(pid_file()?, pid.to_string());
    Ok(pid)
}

/// Attend SOCKS ouvert + bootstrap 100%. Retourne (ok, pct).
pub fn wait_ready(timeout_secs: u64) -> (bool, Option<u32>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut pct = None;
    while std::time::Instant::now() < deadline {
        if !is_running() {
            return (false, bootstrap_pct());
        }
        pct = bootstrap_pct();
        if pct == Some(100) && socks_ready() {
            return (true, Some(100));
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    (socks_ready() && pct.unwrap_or(0) >= 100, pct)
}

pub fn stop() {
    if let Ok(p) = pid_file() {
        let pid: Option<u32> = std::fs::read_to_string(&p).ok().and_then(|t| t.trim().parse().ok());
        let _ = std::fs::remove_file(&p);
        if let Some(pid) = pid {
            kill_pid(pid);
        }
    }
}

/// Installe si besoin, démarre si besoin, attend prêt.
pub fn ensure(log: &dyn Fn(&str)) -> bool {
    if !is_installed() {
        log("[Tor] Tor absent → téléchargement (1x)…");
        match install(&|m| log(m)) {
            Ok(true) => {}
            _ => return false,
        }
    }
    if !is_running() {
        log("[Tor] Démarrage Tor…");
        if start().is_err() {
            return false;
        }
    }
    let (ok, pct) = wait_ready(150);
    let msg = if ok {
        "[Tor] Tor prêt ✓".to_string()
    } else {
        format!("[Tor] Tor pas prêt ({}%)", pct.map(|p| p.to_string()).unwrap_or("?".into()))
    };
    log(&msg);
    ok
}

pub fn status() -> serde_json::Value {
    json!({
        "installed": is_installed(),
        "running": is_running(),
        "socks": socks_ready(),
        "bootstrap": bootstrap_pct(),
    })
}
