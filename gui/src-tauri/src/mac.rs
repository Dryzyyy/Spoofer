use winreg::enums::*;
use winreg::RegKey;

use crate::elevation;
use crate::net::Adapter;
use crate::ps::run_ps;

const REG_CLASS: &str = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e972-e325-11ce-bfc1-08002be10318}";

fn open_class() -> Result<RegKey, String> {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(REG_CLASS)
        .map_err(|_| "clé registre classe réseau introuvable".to_string())
}

/// Retrouve la sous-clé registre d'un adaptateur via son GUID.
pub fn find_reg_key_for_guid(guid: &str) -> Option<String> {
    let g = guid.trim().to_lowercase();
    if g.is_empty() {
        return None;
    }
    let root = open_class().ok()?;
    for name in root.enum_keys().filter_map(|r| r.ok()) {
        if name == "Properties" {
            continue;
        }
        let path = format!("{REG_CLASS}\\{name}");
        let k = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&path).ok()?;
        if let Ok(val) = k.get_value::<String, _>("NetCfgInstanceId") {
            if val.trim().to_lowercase() == g {
                return Some(path);
            }
        }
    }
    None
}

fn find_reg_key_for_name(adapter_name: &str) -> Option<String> {
    let root = open_class().ok()?;
    let needle = adapter_name.to_lowercase();
    for name in root.enum_keys().filter_map(|r| r.ok()) {
        if name == "Properties" {
            continue;
        }
        let path = format!("{REG_CLASS}\\{name}");
        let k = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&path).ok()?;
        let desc: String = k.get_value("DriverDesc").unwrap_or_default();
        if desc.to_lowercase().contains(&needle) {
            return Some(path);
        }
    }
    None
}

fn reg_path_for(a: &Adapter) -> Option<String> {
    if !a.guid.is_empty() {
        if let Some(p) = find_reg_key_for_guid(&a.guid) {
            return Some(p);
        }
    }
    find_reg_key_for_name(&a.name)
}

/// Valeur NetworkAddress (consigne de spoof), None si pas spoofée.
pub fn get_spoofed_mac(a: &Adapter) -> Option<String> {
    let path = reg_path_for(a)?;
    let k = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(&path).ok()?;
    k.get_value::<String, _>("NetworkAddress").ok().map(|v| v.to_uppercase())
}

fn require_admin() -> Result<(), String> {
    if elevation::is_admin() {
        Ok(())
    } else {
        Err("Droits admin requis pour changer la MAC.".into())
    }
}

pub fn set_mac_address(a: &Adapter, new_mac_nosep: &str) -> Result<(), String> {
    require_admin()?;
    let path = reg_path_for(a).ok_or("Clé registre introuvable pour cet adaptateur.")?;
    let k = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(&path, KEY_SET_VALUE)
        .map_err(|e| format!("registre: {e}"))?;
    k.set_value("NetworkAddress", &new_mac_nosep.to_uppercase())
        .map_err(|e| format!("registre: {e}"))?;
    restart_adapter(&a.name);
    Ok(())
}

pub fn clear_mac_address(a: &Adapter) -> Result<(), String> {
    require_admin()?;
    let path = reg_path_for(a).ok_or("Clé registre introuvable pour cet adaptateur.")?;
    let k = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(&path, KEY_SET_VALUE)
        .map_err(|e| format!("registre: {e}"))?;
    // Absence de valeur = pas de spoof (FileNotFound ignoré comme en Python)
    match k.delete_value("NetworkAddress") {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {}
    }
    restart_adapter(&a.name);
    Ok(())
}

fn restart_adapter(name: &str) {
    let safe = name.replace('\'', "''");
    let (code, _, _) = run_ps(&format!("Restart-NetAdapter -Name '{safe}' -Confirm:$false"));
    if code != 0 {
        run_ps(&format!("Disable-NetAdapter -Name '{safe}' -Confirm:$false"));
        std::thread::sleep(std::time::Duration::from_secs(2));
        run_ps(&format!("Enable-NetAdapter -Name '{safe}' -Confirm:$false"));
    }
}
