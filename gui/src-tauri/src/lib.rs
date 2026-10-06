mod dbg;
mod elevation;
mod fetch;
mod mac;
mod net;
mod paths;
mod proxy;
mod ps;
mod settings;
mod tor;
mod tun;
#[cfg(test)]
mod unit_tests;

use serde_json::{Map, Value};
use tauri::Emitter;

fn emit(app: &tauri::AppHandle, msg: &str) {
    let _ = app.emit("ghostnet://log", msg.to_string());
}

// ---------- helpers ----------

fn mac_display(
    a: &net::Adapter,
    originals: &std::collections::HashMap<String, settings::Original>,
) -> (String, String, Option<String>, bool) {
    let effective = a.mac.to_uppercase();
    let raw_spoof = mac::get_spoofed_mac(a);
    let spoof = raw_spoof.map(|r| net::dash_mac(&r));
    let mut origine = originals.get(&a.name).map(|o| o.real.clone()).unwrap_or_default();
    if spoof.is_some() {
        if origine.is_empty() {
            origine = "inconnue".into();
        }
        (effective, origine, spoof, true)
    } else {
        (effective.clone(), effective, None, false)
    }
}

fn compute_masked(adapters: &[net::Adapter]) -> bool {
    let (ena, _) = proxy::get_system_proxy();
    ena || tun::is_running() || adapters.iter().any(|a| mac::get_spoofed_mac(a).is_some())
}

/// Un seul passage : énumère les adaptateurs UNE fois et construit
/// statut + liste. Évite de lancer 4 powershell en parallèle à chaque refresh.
fn dashboard_value(include_public_ip: bool) -> Value {
    // Les 3 étapes lentes tournent en parallèle : sur cette machine
    // adapters ≈ 3.1s (powershell) et tor.socks ≈ 2.1s (TCP localhost lent
    // à refuser) ; en séquentiel = 5.6s de gel ressenti, en parallèle ≈ 3.2s.
    // Intervalle (with_ip=false) : adaptateurs cachés 45s → vagues ~1s.
    // Manuel/ON/OFF (with_ip=true) : énumération fraîche.
    let fresh = include_public_ip;
    let (adapters, tor_status, public_ip): (Vec<net::Adapter>, Value, Option<String>) =
        std::thread::scope(|scope| {
            let h_ad = scope.spawn(move || {
                if fresh {
                    net::get_adapters()
                } else {
                    net::get_adapters_cached()
                }
            });
            let h_tor = scope.spawn(tor::status);
            let h_ip = if include_public_ip {
                Some(scope.spawn(net::get_public_ip))
            } else {
                None
            };
            (
                h_ad.join().unwrap_or_default(),
                h_tor.join().unwrap_or(serde_json::json!({
                    "installed": false, "running": false, "socks": false, "bootstrap": null
                })),
                h_ip.and_then(|h| h.join().ok()),
            )
        });
    let originals = settings::load_originals();
    let (pena, psrv) = proxy::get_system_proxy();
    let s = settings::load_settings();
    let masked = compute_masked(&adapters);
    let target = net::pick_target_adapter(&adapters, &settings::s_str(&s, "mac_iface")).map(|t| {
        let (eff, ori, spoof, spoofed) = mac_display(t, &originals);
        serde_json::json!({
            "name": t.name, "effective": eff, "origine": ori,
            "spoof": spoof, "spoofed": spoofed, "ip": t.ip, "status": t.status,
        })
    });
    let list: Vec<Value> = adapters
        .iter()
        .map(|a| {
            let (eff, ori, spoof, spoofed) = mac_display(a, &originals);
            serde_json::json!({
                "name": a.name, "desc": a.desc, "mac": a.mac,
                "effective": eff, "origine": ori, "spoof": spoof, "spoofed": spoofed,
                "ip": a.ip, "status": a.status, "guid": a.guid, "ifindex": a.ifindex,
            })
        })
        .collect();
    let mut v = serde_json::json!({
        "status": {
            "masked": masked,
            "admin": elevation::is_admin(),
            "proxy": {"enabled": pena, "server": psrv},
            "tun": {"installed": tun::is_installed(), "running": tun::is_running()},
            "tor": tor_status,
            "settings": Value::Object(s),
            "target": target,
        },
        "adapters": Value::Array(list),
    });
    if let Some(ip) = public_ip {
        v["publicIp"] = Value::String(ip);
    }
    v
}

fn status_value(include_public_ip: bool) -> Value {
    let d = dashboard_value(false);
    let status = d.get("status").cloned().unwrap_or(Value::Null);
    if include_public_ip {
        let mut s = status;
        s["public_ip"] = Value::String(net::get_public_ip());
        s
    } else {
        status
    }
}

// ---------- commandes ----------

#[tauri::command]
fn is_admin() -> bool {
    elevation::is_admin()
}

/// Plus de sidecar : toujours true (compat frontend).
#[tauri::command]
fn core_file_exists() -> bool {
    true
}

#[tauri::command]
fn spoofer_dir() -> Result<String, String> {
    Ok(paths::spoofer_dir()?.to_string_lossy().to_string())
}

#[tauri::command]
fn get_status(include_public_ip: Option<bool>) -> Result<Value, String> {
    Ok(status_value(include_public_ip.unwrap_or(false)))
}

#[tauri::command]
fn get_adapters() -> Result<Value, String> {
    Ok(dashboard_value(false)["adapters"].clone())
}

/// Refresh complet en UN appel (statut + adaptateurs + IP optionnelle).
/// Le frontend doit préférer cette commande : 2 powershell au lieu de 4+.
#[tauri::command]
fn get_dashboard(with_ip: Option<bool>) -> Result<Value, String> {
    let t0 = std::time::Instant::now();
    dbg::dlog(&format!("get_dashboard(with_ip={}) entrée", with_ip.unwrap_or(false)));
    let v = dashboard_value(with_ip.unwrap_or(false));
    let n = v.get("adapters").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0);
    dbg::dlog(&format!("get_dashboard OK en {}ms ({} adaptateurs)", t0.elapsed().as_millis(), n));
    Ok(v)
}

#[tauri::command]
fn get_public_ip() -> Result<String, String> {
    Ok(net::get_public_ip())
}

#[tauri::command]
fn get_settings() -> Result<Value, String> {
    Ok(Value::Object(settings::load_settings()))
}

#[tauri::command]
fn save_settings(settings: Value) -> Result<String, String> {
    match settings {
        Value::Object(o) => {
            settings::save_settings(&o)?;
            Ok("ok".into())
        }
        _ => Err("settings invalides".into()),
    }
}

#[tauri::command]
fn tun_status() -> Result<Value, String> {
    Ok(serde_json::json!({"installed": tun::is_installed(), "running": tun::is_running()}))
}

#[tauri::command]
fn tun_stop() -> Result<String, String> {
    tun::stop(false);
    Ok("tun OFF".into())
}

#[tauri::command]
fn tor_status() -> Result<Value, String> {
    Ok(tor::status())
}

// ---------- MAC ----------

fn memorize_original(a: &net::Adapter) -> Result<(), String> {
    let mut sv = settings::load_originals();
    if !sv.contains_key(&a.name) && mac::get_spoofed_mac(a).is_none() {
        sv.insert(
            a.name.clone(),
            settings::Original { real: a.mac.to_uppercase(), guid: a.guid.clone() },
        );
        settings::store_originals(&sv)?;
    }
    Ok(())
}

#[tauri::command]
async fn mac_apply(
    app: tauri::AppHandle,
    iface: String,
    random: Option<bool>,
    fixed: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = net::get_adapters();
        let tgt = net::pick_target_adapter(&adapters, &iface).ok_or("carte introuvable")?.clone();
        let nosep = match fixed.map(|f| f.trim().to_string()).filter(|f| !f.is_empty()) {
            Some(f) => {
                if !net::valid_mac_nosep(&f) {
                    return Err("MAC fixe invalide (ex: 02:11:22:33:44:55).".to_string());
                }
                net::clean_mac(&f)
            }
            None => {
                if !random.unwrap_or(true) {
                    // fixe demandée via settings
                    let s = settings::load_settings();
                    let c = settings::s_str(&s, "custom_mac");
                    if c.trim().is_empty() {
                        net::clean_mac(&net::random_mac_colon())
                    } else if !net::valid_mac_nosep(&c) {
                        return Err("MAC fixe invalide (ex: 02:11:22:33:44:55).".to_string());
                    } else {
                        net::clean_mac(&c)
                    }
                } else {
                    net::clean_mac(&net::random_mac_colon())
                }
            }
        };
        memorize_original(&tgt)?;
        emit(&app, &format!("[MAC] {} → {}… (coupure ~5s)", tgt.name, net::dash_mac(&nosep)));
        mac::set_mac_address(&tgt, &nosep)?;
        emit(&app, "[MAC] spoofée ✓ — renouvellement DHCP…");
        let rip = net::wait_for_valid_ip(&tgt.name, 30);
        if rip.starts_with("169.254.") || rip == "-" {
            emit(&app, &format!("[IP locale] ⚠ {} = {rip} (APIPA : patiente 30s puis Actualiser)", tgt.name));
        } else {
            emit(&app, &format!("[IP locale] {} = {rip} ✓", tgt.name));
        }
        Ok("ok".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn mac_restore(app: tauri::AppHandle, iface: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = net::get_adapters();
        let targets: Vec<net::Adapter> = match adapters.iter().find(|a| a.name == iface) {
            Some(a) => vec![a.clone()],
            None => adapters.clone(),
        };
        for a in &targets {
            if mac::get_spoofed_mac(a).is_some() {
                mac::clear_mac_address(a)?;
                emit(&app, &format!("[MAC] {} restaurée — renouvellement DHCP…", a.name));
                let rip = net::wait_for_valid_ip(&a.name, 30);
                emit(&app, &format!("[IP locale] {} = {rip}", a.name));
            }
        }
        Ok("ok".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------- ON / OFF ----------

fn do_mask_on(app: &tauri::AppHandle) -> Result<(), String> {
    let s: Map<String, Value> = settings::load_settings();
    let g = |k: &str| settings::s_str(&s, k);
    let adapters = net::get_adapters();
    let admin = elevation::is_admin();

    if !admin {
        emit(app, "[MAC/IP] non-admin : MAC + TUN exigent admin. Le proxy seul sera appliqué.");
    }

    // 1) MAC
    let tgt = net::pick_target_adapter(&adapters, &g("mac_iface")).cloned();
    if let Some(t) = tgt {
        if admin {
            let raw = {
                let c = g("custom_mac");
                if c.trim().is_empty() { net::random_mac_colon() } else { c }
            };
            if !net::valid_mac_nosep(&raw) {
                return Err("MAC fixe invalide (ex: 02:11:22:33:44:55).".into());
            }
            let nosep = net::clean_mac(&raw);
            memorize_original(&t)?;
            emit(app, &format!("[MAC] {} → {}… (coupure ~5s)", t.name, net::dash_mac(&nosep)));
            mac::set_mac_address(&t, &nosep)?;
            emit(app, "[MAC] spoofée ✓ — renouvellement DHCP…");
            let rip = net::wait_for_valid_ip(&t.name, 30);
            if rip.starts_with("169.254.") || rip == "-" {
                emit(app, &format!("[IP locale] ⚠ {} = {rip} (APIPA : patiente 30s puis Actualiser, ou coupe/rallume le Wi-Fi)", t.name));
            } else {
                emit(app, &format!("[IP locale] {} = {rip} ✓", t.name));
            }
        } else {
            emit(app, "[MAC] sautée (pas admin).");
        }
    }

    // 2) IP selon méthode
    let method = g("ip_method");
    let srv = proxy::proxy_server_string(&g("ptype"), &g("phost"), &g("pport"));
    let bypass = net::parse_bypass(&g("bypass_apps"));
    if (method == "tun_proxy" || method == "tun_wg") && !admin {
        return Err("Le TUN exige les droits administrateur : relance l'app en admin. Sans admin, seul le mode « Proxy seul » fonctionne.".into());
    }
    if method == "tun_wg" {
        let path = g("wg_path");
        if path.trim().is_empty() || !std::path::Path::new(&path).exists() {
            return Err("Conf WireGuard introuvable — choisis un .conf dans ⚙.".into());
        }
        let wg = tun::parse_wg_conf(&path)?;
        tun::write_config(&tun::build_wg_config(&wg, &bypass))?;
        tun::check_config()?;
        let pid = tun::start()?;
        let _ = proxy::disable_system_proxy();
        emit(app, &format!("[TUN] WireGuard {} (pid {pid}) — tout l'OS ✓", wg.endpoint_host));
    } else if method == "tun_proxy" {
        if !tun::is_installed() {
            emit(app, "[TUN] moteur absent → installation…");
            tun::install(&|m| emit(app, m))?;
        }
        let srv = srv.ok_or("Proxy Hôte/Port vides — règle-les dans ⚙.")?;
        let h = g("phost");
        let p: u16 = g("pport").parse().map_err(|_| "Port proxy invalide.")?;
        if !net::proxy_reachable(&h, p, 5) {
            if (h == "127.0.0.1" || h == "localhost") && p == 9050 {
                emit(app, "[Tor] démarrage auto (1ʳᵉ fois : téléchargement officiel)…");
                if !tor::ensure(&|m| emit(app, m)) {
                    return Err("Tor n'a pas démarré (voir bin/tor/tor.log).".into());
                }
                emit(app, "[Tor] prêt ✓");
            }
            if !net::proxy_reachable(&h, p, 5) {
                return Err(format!("Proxy {h}:{p} injoignable — lance Tor / ton proxy AVANT. Sans ça, le TUN couperait tout le réseau. (MAC déjà spoofée, IP non masquée)"));
            }
        }
        tun::write_config(&tun::build_socks_config(&h, p, "", "", &bypass))?;
        tun::check_config()?;
        let pid = tun::start()?;
        proxy::set_system_proxy(&srv)?;
        if bypass.is_empty() {
            emit(app, &format!("[TUN] tout l'OS → {h}:{p} (pid {pid}) ✓"));
        } else {
            emit(app, &format!("[TUN] tout l'OS → {h}:{p} (pid {pid}) ✓ — sauf {} (direct)", bypass.join(", ")));
        }
    } else {
        let srv = srv.ok_or("Proxy Hôte/Port vides — règle-les dans ⚙.")?;
        proxy::set_system_proxy(&srv)?;
        emit(app, &format!("[IP] proxy {srv} ✓ (navigateurs)"));
    }
    Ok(())
}

fn do_mask_off(app: &tauri::AppHandle) {
    tun::stop(true);
    emit(app, "[TUN] stoppé.");
    if tor::is_running() {
        tor::stop();
        emit(app, "[Tor] arrêté.");
    }
    let _ = proxy::disable_system_proxy();
    emit(app, "[IP] trafic direct.");
    if elevation::is_admin() {
        for a in net::get_adapters() {
            if mac::get_spoofed_mac(&a).is_some() {
                match mac::clear_mac_address(&a) {
                    Ok(()) => {
                        emit(app, &format!("[MAC] {} restaurée — renouvellement DHCP…", a.name));
                        let rip = net::wait_for_valid_ip(&a.name, 30);
                        if rip != "-" && !rip.starts_with("169.254.") {
                            emit(app, &format!("[IP locale] {} = {rip} ✓", a.name));
                        } else {
                            emit(app, &format!("[IP locale] {} = {rip} ⚠ (APIPA, patiente puis Actualiser)", a.name));
                        }
                    }
                    Err(e) => emit(app, &format!("[MAC ERR {}] {e}", a.name)),
                }
            }
        }
    } else {
        emit(app, "[MAC] restauration impossible sans admin.");
    }
}

#[tauri::command]
async fn mask_on(app: tauri::AppHandle) -> Result<String, String> {
    dbg::dlog("mask_on entrée");
    let r = tauri::async_runtime::spawn_blocking(move || do_mask_on(&app))
        .await
        .map_err(|e| e.to_string())?;
    match &r {
        Ok(()) => dbg::dlog("mask_on OK"),
        Err(e) => dbg::dlog(&format!("mask_on ERR: {e}")),
    }
    r?;
    Ok("ok".into())
}

#[tauri::command]
async fn mask_off(app: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || do_mask_off(&app))
        .await
        .map_err(|e| e.to_string())?;
    Ok("ok".into())
}

#[tauri::command]
async fn tun_install(app: tauri::AppHandle) -> Result<String, String> {
    let ok = tauri::async_runtime::spawn_blocking(move || tun::install(&|m| emit(&app, m)))
        .await
        .map_err(|e| e.to_string())??;
    Ok(if ok { "ok".into() } else { "Échec install.".into() })
}

#[tauri::command]
fn relaunch_as_admin() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-Command", &format!("Start-Process '{}' -Verb RunAs", exe.to_string_lossy())]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok("ok".into())
}

/// Boucle de polling sur thread dédié : le frontend ne fait QUE recevoir
/// `ghostnet://state` (zéro setInterval, zéro invoke périodique côté JS).
/// Tourne même pendant un masquage (peu coûteux : API native + 1 TCP),
/// donc l'UI bascule PROTÉGÉ/EXPOSÉ en direct.
fn poll_loop(app: tauri::AppHandle) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(15));
        let v = dashboard_value(false);
        let _ = app.emit("ghostnet://state", v);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            let handle = app.handle().clone();
            std::thread::spawn(move || poll_loop(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            is_admin,
            core_file_exists,
            spoofer_dir,
            get_status,
            get_adapters,
            get_dashboard,
            get_public_ip,
            get_settings,
            save_settings,
            tun_status,
            tun_stop,
            tor_status,
            mac_apply,
            mac_restore,
            mask_on,
            mask_off,
            tun_install,
            relaunch_as_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
