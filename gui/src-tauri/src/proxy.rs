use winreg::enums::*;
use winreg::RegKey;

const REG_PROXY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";

pub fn get_system_proxy() -> (bool, String) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let k = match hkcu.open_subkey(REG_PROXY) {
        Ok(k) => k,
        Err(_) => return (false, String::new()),
    };
    let ena: u32 = k.get_value("ProxyEnable").unwrap_or(0);
    let srv: String = k.get_value("ProxyServer").unwrap_or_default();
    (ena != 0, srv)
}

fn open_writable() -> Result<RegKey, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey_with_flags(REG_PROXY, KEY_SET_VALUE)
        .map_err(|e| format!("registre proxy: {e}"))
}

pub fn set_system_proxy(server: &str) -> Result<(), String> {
    let k = open_writable()?;
    k.set_value("ProxyEnable", &1u32).map_err(|e| e.to_string())?;
    k.set_value("ProxyServer", &server.to_string()).map_err(|e| e.to_string())?;
    // bypass local/LAN comme l'ancien Python
    let _ = k.set_value(
        "ProxyOverride",
        &"localhost;127.*;10.*;192.168.*;*.local".to_string(),
    );
    refresh_wininet();
    Ok(())
}

pub fn disable_system_proxy() -> Result<(), String> {
    let k = open_writable()?;
    k.set_value("ProxyEnable", &0u32).map_err(|e| e.to_string())?;
    refresh_wininet();
    Ok(())
}

fn refresh_wininet() {
    use windows::Win32::Networking::WinInet::InternetSetOptionW;
    unsafe {
        // INTERNET_OPTION_SETTINGS_CHANGED = 39, INTERNET_OPTION_REFRESH = 37
        let _ = InternetSetOptionW(None, 39, None, 0);
        let _ = InternetSetOptionW(None, 37, None, 0);
    }
}

pub fn proxy_server_string(ptype: &str, host: &str, port: &str) -> Option<String> {
    let h = host.trim();
    let p = port.trim();
    if h.is_empty() || p.is_empty() {
        return None;
    }
    if ptype.contains("SOCKS") {
        Some(format!("socks={h}:{p}"))
    } else {
        Some(format!("{h}:{p}"))
    }
}
