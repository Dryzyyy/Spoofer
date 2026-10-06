use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::fetch::download;
use crate::paths::spoofer_dir;
use crate::ps::{kill_pid, pid_alive, run_quiet_timeout};

pub const SINGBOX_VERSION: &str = "1.10.7";
pub fn singbox_url() -> String {
    format!(
        "https://github.com/SagerNet/sing-box/releases/download/v{SINGBOX_VERSION}/sing-box-{SINGBOX_VERSION}-windows-amd64.zip"
    )
}
pub const WINTUN_URL: &str = "https://www.wintun.net/builds/wintun-0.14.1.zip";

fn bin_dir() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("bin"))
}
fn singbox_exe() -> Result<PathBuf, String> {
    Ok(bin_dir()?.join("sing-box.exe"))
}
fn wintun_dll() -> Result<PathBuf, String> {
    Ok(bin_dir()?.join("wintun.dll"))
}
fn config_path() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("sing-tun.json"))
}
fn pid_path() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("sing-tun.pid"))
}
fn log_path() -> Result<PathBuf, String> {
    Ok(spoofer_dir()?.join("sing-tun.log"))
}

pub fn is_installed() -> bool {
    singbox_exe().map(|p| p.exists()).unwrap_or(false)
}

fn hide(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
}

/// Télécharge sing-box + wintun. log(texte) pour le journal GUI.
pub fn install(log: &dyn Fn(&str)) -> Result<bool, String> {
    let bin = bin_dir()?;
    std::fs::create_dir_all(&bin).map_err(|e| format!("bin/: {e}"))?;
    log("[TUN] Téléchargement sing-box…");
    let tmp = std::env::temp_dir().join("singbox.zip");
    download(&singbox_url(), &tmp, log, "sing-box", 300)?;
    // extrait sing-box.exe
    {
        let f = File::open(&tmp).map_err(|e| format!("zip: {e}"))?;
        let mut zip = zip::ZipArchive::new(f).map_err(|e| format!("zip: {e}"))?;
        let mut done = false;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| format!("zip: {e}"))?;
            let name = entry.name().to_owned();
            if name.to_lowercase().ends_with("sing-box.exe") && !entry.is_dir() {
                let dest = singbox_exe()?;
                let mut out = File::create(&dest).map_err(|e| format!("écriture sing-box: {e}"))?;
                std::io::copy(&mut entry, &mut out).map_err(|e| format!("extraction: {e}"))?;
                done = true;
                break;
            }
        }
        if !done {
            return Err("sing-box.exe introuvable dans l'archive".into());
        }
    }
    let _ = std::fs::remove_file(&tmp);
    log("[TUN] Téléchargement wintun…");
    let tmp2 = std::env::temp_dir().join("wintun.zip");
    match download(WINTUN_URL, &tmp2, &|_| {}, "wintun", 120) {
        Ok(()) => {
            let f = File::open(&tmp2).map_err(|e| format!("zip wintun: {e}"))?;
            if let Ok(mut zip) = zip::ZipArchive::new(f) {
                for i in 0..zip.len() {
                    if let Ok(mut entry) = zip.by_index(i) {
                        let name = entry.name().to_owned();
                        if name.to_lowercase().ends_with("amd64/wintun.dll") && !entry.is_dir() {
                            if let Ok(dest) = wintun_dll() {
                                if let Ok(mut out) = File::create(&dest) {
                                    let _ = std::io::copy(&mut entry, &mut out);
                                }
                            }
                            break;
                        }
                    }
                }
            }
            let _ = std::fs::remove_file(&tmp2);
        }
        Err(e) => log(&format!("[TUN] wintun optionnel ignoré: {e}")),
    }
    let ok = is_installed();
    log(if ok { "[TUN] prêt ✓" } else { "[TUN] échec install." });
    Ok(ok)
}

// ---------- WireGuard .conf ----------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WgConfig {
    pub private_key: String,
    pub addresses: Vec<String>,
    pub dns: Vec<String>,
    pub public_key: String,
    pub preshared: String,
    pub endpoint_host: String,
    pub endpoint_port: u16,
    pub allowed_ips: Vec<String>,
    pub keepalive: i64,
    pub mtu: i64,
}

pub fn parse_wg_conf(path: &str) -> Result<WgConfig, String> {
    let txt = std::fs::read_to_string(path).map_err(|e| format!("lecture .conf: {e}"))?;
    let mut sec = "";
    let mut iface: std::collections::HashMap<String, String> = Default::default();
    let mut peer: std::collections::HashMap<String, String> = Default::default();
    for raw in txt.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.eq_ignore_ascii_case("[interface]") {
            sec = "if";
            continue;
        }
        if line.eq_ignore_ascii_case("[peer]") {
            sec = "peer";
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let kl = k.trim().to_lowercase();
            let vv = v.trim().to_string();
            if sec == "if" {
                iface.insert(kl, vv);
            } else if sec == "peer" {
                peer.insert(kl, vv);
            }
        }
    }
    let pk = iface.get("privatekey").cloned().unwrap_or_default();
    let pubk = peer.get("publickey").cloned().unwrap_or_default();
    let ep = peer.get("endpoint").cloned().unwrap_or_default();
    if pk.is_empty() || pubk.is_empty() || ep.is_empty() {
        return Err("Conf incomplet : PrivateKey / PublicKey / Endpoint requis.".into());
    }
    let (host, port) = parse_endpoint(&ep)?;
    let addrs: Vec<String> = iface
        .get("address")
        .map(|s| s.as_str())
        .unwrap_or("10.64.0.2/32")
        .split(',')
        .map(|a| {
            let a = a.trim().to_string();
            if a.contains('/') { a } else { format!("{a}/32") }
        })
        .collect();
    let allowed: Vec<String> = peer
        .get("allowedips")
        .map(|s| s.as_str())
        .unwrap_or("0.0.0.0/0, ::/0")
        .split(',')
        .map(|a| a.trim().to_string())
        .collect();
    let dns: Vec<String> = iface
        .get("dns")
        .map(|s| {
            s.split(',')
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Ok(WgConfig {
        private_key: pk,
        addresses: addrs,
        dns,
        public_key: pubk,
        preshared: peer.get("presharedkey").cloned().unwrap_or_default(),
        endpoint_host: host,
        endpoint_port: port,
        allowed_ips: allowed,
        keepalive: peer.get("persistentkeepalive").and_then(|s| s.parse().ok()).unwrap_or(25),
        mtu: iface.get("mtu").and_then(|s| s.parse().ok()).unwrap_or(1280),
    })
}

fn parse_endpoint(ep: &str) -> Result<(String, u16), String> {
    let ep = ep.trim();
    // [ipv6]:port
    if ep.starts_with('[') {
        if let Some(end) = ep.find("]:") {
            let host = ep[1..end].to_string();
            let port: u16 = ep[end + 2..].parse().map_err(|_| "port endpoint invalide")?;
            return Ok((host, port));
        }
        return Err("endpoint IPv6 invalide (attendu [ip]:port)".into());
    }
    match ep.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p.parse().map_err(|_| "port endpoint invalide")?;
            Ok((h.to_string(), port))
        }
        None => Err("endpoint invalide (attendu host:port)".into()),
    }
}

// ---------- Configs sing-box ----------

fn tun_inbound() -> Value {
    json!({
        "type": "tun",
        "tag": "tun-in",
        "interface_name": "GhostTUN",
        "address": ["172.18.0.1/30"],
        "mtu": 9000,
        "stack": "mixed",
        "auto_route": true,
        "strict_route": true,
        "sniff": true,
        "route_exclude_address": [
            "192.168.0.0/16", "10.0.0.0/8", "172.16.0.0/12", "127.0.0.0/8", "169.254.0.0/16"
        ],
    })
}

fn dns_section(detour: &str, exclude: &[String]) -> Value {
    let mut rules = vec![];
    if !exclude.is_empty() {
        rules.push(json!({"process_name": exclude, "server": "direct-dns"}));
    }
    rules.push(json!({"outbound": "any", "server": "proxy-dns"}));
    json!({
        "servers": [
            {"tag": "proxy-dns", "address": "https://1.1.1.1/dns-query", "detour": detour},
            {"tag": "direct-dns", "address": "223.5.5.5", "detour": "direct"},
        ],
        "rules": rules,
        "final": "proxy-dns",
    })
}

fn route_section(final_tag: &str, exclude: &[String]) -> Value {
    let mut rules = vec![];
    if !exclude.is_empty() {
        rules.push(json!({"process_name": exclude, "outbound": "direct"}));
    }
    rules.push(json!({"protocol": "dns", "outbound": "dns-out"}));
    let mut route = serde_json::Map::new();
    route.insert("rules".into(), Value::Array(rules));
    route.insert("final".into(), Value::String(final_tag.into()));
    route.insert("auto_detect_interface".into(), Value::Bool(true));
    if !exclude.is_empty() {
        route.insert("find_process".into(), Value::Bool(true));
    }
    Value::Object(route)
}

pub fn build_socks_config(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    exclude: &[String],
) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("type".into(), "socks".into());
    out.insert("tag".into(), "proxy".into());
    out.insert("server".into(), host.into());
    out.insert("server_port".into(), port.into());
    out.insert("version".into(), 5.into());
    if !user.is_empty() {
        out.insert("username".into(), user.into());
        out.insert("password".into(), password.into());
    }
    json!({
        "log": {"level": "warn"},
        "dns": dns_section("proxy", exclude),
        "inbounds": [tun_inbound()],
        "outbounds": [Value::Object(out), {"type": "direct", "tag": "direct"}, {"type": "dns", "tag": "dns-out"}],
        "route": route_section("proxy", exclude),
    })
}

pub fn build_wg_config(wg: &WgConfig, exclude: &[String]) -> Value {
    let mut peer = serde_json::Map::new();
    peer.insert("server".into(), wg.endpoint_host.clone().into());
    peer.insert("server_port".into(), wg.endpoint_port.into());
    peer.insert("public_key".into(), wg.public_key.clone().into());
    peer.insert("allowed_ips".into(), wg.allowed_ips.clone().into());
    if !wg.preshared.is_empty() {
        peer.insert("pre_shared_key".into(), wg.preshared.clone().into());
    }
    json!({
        "log": {"level": "warn"},
        "dns": dns_section("proxy", exclude),
        "inbounds": [tun_inbound()],
        "outbounds": [{
            "type": "wireguard",
            "tag": "proxy",
            "local_address": wg.addresses,
            "private_key": wg.private_key,
            "peers": [Value::Object(peer)],
            "mtu": wg.mtu,
        }, {"type": "direct", "tag": "direct"}, {"type": "dns", "tag": "dns-out"}],
        "route": route_section("proxy", exclude),
    })
}

pub fn write_config(v: &Value) -> Result<PathBuf, String> {
    let p = config_path()?;
    let txt = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    std::fs::write(&p, txt).map_err(|e| format!("écriture sing-tun.json: {e}"))?;
    Ok(p)
}

pub fn check_config() -> Result<String, String> {
    if !is_installed() {
        return Err("Moteur TUN non installé (bin/sing-box.exe manquant).".into());
    }
    let exe = singbox_exe()?;
    let cfg = config_path()?;
    let mut cmd = Command::new(&exe);
    cmd.args(["check", "-c"]);
    cmd.arg(&cfg);
    hide(&mut cmd);
    let o = cmd.output().map_err(|e| format!("sing-box check: {e}"))?;
    let msg = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let tail: String = msg.chars().rev().take(2000).collect::<String>().chars().rev().collect();
    if o.status.success() {
        Ok(if tail.trim().is_empty() { "OK".into() } else { tail.trim().to_string() })
    } else {
        Err(format!("Config refusée : {}", tail.trim()))
    }
}

// ---------- run/stop ----------

pub fn is_running() -> bool {
    let p = match pid_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let txt = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => return false,
    };
    match txt.trim().parse::<u32>() {
        Ok(pid) => pid_alive(pid),
        Err(_) => false,
    }
}

pub fn start() -> Result<u32, String> {
    if !is_installed() {
        return Err("Moteur non installé. Clique 'Installer moteur TUN'.".into());
    }
    check_config().map_err(|e| format!("Config refusée : {e}"))?;
    stop(true);
    let logp = log_path()?;
    {
        let mut f = File::options().create(true).append(true).open(&logp).map_err(|e| e.to_string())?;
        let _ = writeln!(f, "\n--- démarrage ---");
    }
    let exe = singbox_exe()?;
    let cfg = config_path()?;
    let mut cmd = Command::new(&exe);
    cmd.args(["run", "-c"]);
    cmd.arg(&cfg);
    let logf = File::options().create(true).append(true).open(&logp).map_err(|e| e.to_string())?;
    let logf2 = logf.try_clone().map_err(|e| e.to_string())?;
    cmd.stdout(Stdio::from(logf));
    cmd.stderr(Stdio::from(logf2));
    hide(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| format!("lancement sing-box: {e}"))?;
    // Vérifie que le processus survit à l'init (~15s, 1er GhostTUN = install driver)
    let mut dead = false;
    for _ in 0..15 {
        std::thread::sleep(Duration::from_secs(1));
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => {
                dead = true;
                break;
            }
            None => {}
        }
    }
    if dead {
        let tail = std::fs::read_to_string(&logp).unwrap_or_default();
        let tail: String = tail.chars().rev().take(1500).collect::<String>().chars().rev().collect();
        if tail.contains("Access is denied") {
            return Err("TUN refusé par Windows (Access denied) : relance l'app EN ADMINISTRATEUR via run-admin.bat. Sans admin, le pilote TUN ne peut pas se créer.".into());
        }
        let short: String = tail.chars().rev().take(500).collect::<String>().chars().rev().collect();
        return Err(format!("Le TUN est mort au démarrage : {}", if short.trim().is_empty() { "voir sing-tun.log" } else { short.trim() }));
    }
    let pid = child.id();
    if let Ok(p) = pid_path() {
        let _ = std::fs::write(&p, pid.to_string());
    }
    // détache : on oublie l'enfant (le pidfile le suit)
    std::mem::forget(child);
    run_quiet_timeout("ipconfig", &["/flushdns"], 20);
    Ok(pid)
}

pub fn stop(silent: bool) {
    let mut pid: Option<u32> = None;
    if let Ok(p) = pid_path() {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            pid = txt.trim().parse::<u32>().ok();
        }
        let _ = std::fs::remove_file(&p);
    }
    if let Some(p) = pid {
        kill_pid(p);
    } else if !silent {
        run_quiet_timeout("taskkill", &["/F", "/IM", "sing-box.exe"], 20);
    }
    run_quiet_timeout("ipconfig", &["/flushdns"], 20);
}

#[allow(dead_code)]
pub fn read_log_tail(n: usize) -> String {
    let txt = log_path()
        .and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
        .unwrap_or_default();
    txt.chars().rev().take(n).collect::<String>().chars().rev().collect()
}
