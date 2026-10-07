use rand::Rng;
use serde_json::Value;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::ps::run_ps;

#[derive(Debug, Clone)]
pub struct Adapter {
    pub name: String,
    pub desc: String,
    pub mac: String,
    pub status: String,
    pub guid: String,
    pub ifindex: Option<i64>,
    pub ip: String,
}

fn str_field(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(x) = v.get(*k).and_then(|x| x.as_str()) {
            return x.to_string();
        }
    }
    // variantes de casse (PowerShell peut renvoyer interfaceGuid / ifIndex...)
    if let Some(obj) = v.as_object() {
        for k in keys {
            let lk = k.to_lowercase();
            for (ok, ov) in obj {
                if ok.to_lowercase() == lk {
                    if let Some(s) = ov.as_str() {
                        return s.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

fn int_field(v: &Value, keys: &[&str]) -> Option<i64> {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(|x| x.as_i64()) {
            return Some(n);
        }
    }
    if let Some(obj) = v.as_object() {
        for k in keys {
            let lk = k.to_lowercase();
            for (ok, ov) in obj {
                if ok.to_lowercase() == lk {
                    if let Some(n) = ov.as_i64() {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

fn as_array(v: Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a,
        Value::Null => vec![],
        single => {
            // ConvertTo-Json d'un scalaire/chaîne vide -> ignore
            if single.as_str().map(|s| s.trim().is_empty()).unwrap_or(false) {
                vec![]
            } else {
                vec![single]
            }
        }
    }
}

fn parse_adapters(arr: Value) -> Vec<Adapter> {
    as_array(arr)
        .into_iter()
        .map(|a| Adapter {
            name: str_field(&a, &["Name"]).trim().to_string(),
            desc: str_field(&a, &["InterfaceDescription"]),
            mac: str_field(&a, &["MacAddress"]).to_uppercase(),
            status: str_field(&a, &["Status"]),
            guid: str_field(&a, &["InterfaceGuid"]),
            ifindex: int_field(&a, &["IfIndex", "InterfaceIndex"]),
            ip: "-".into(),
        })
        .filter(|a| !a.name.is_empty() && a.name != "?")
        .collect()
}

fn apply_ipmap(adapters: &mut [Adapter], ips: Value) {
    use std::collections::HashMap;
    let mut ipmap: HashMap<i64, String> = HashMap::new();
    for e in as_array(ips) {
        let idx = int_field(&e, &["InterfaceIndex"]).unwrap_or(-1);
        let ip = str_field(&e, &["IPAddress"]);
        if ip.is_empty() || ip.starts_with("127.") || idx < 0 {
            continue;
        }
        match ipmap.get(&idx) {
            Some(cur) if cur.starts_with("169.254.") => {
                ipmap.insert(idx, ip);
            }
            None => {
                ipmap.insert(idx, ip);
            }
            _ => {}
        }
    }
    for a in adapters.iter_mut() {
        if let Some(idx) = a.ifindex {
            if let Some(ip) = ipmap.get(&idx) {
                a.ip = ip.clone();
            }
        }
    }
}

static AD_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<Adapter>)>> =
    std::sync::Mutex::new(None);

/// Version cachée (TTL 45s) pour le polling d'intervalle : évite de payer
/// ~3s de powershell toutes les 15s. Les actions explicites (ON/OFF,
/// Actualiser) utilisent get_adapters() frais.
pub fn get_adapters_cached() -> Vec<Adapter> {
    if let Ok(guard) = AD_CACHE.lock() {
        if let Some((t, ads)) = guard.as_ref() {
            if t.elapsed().as_secs() < 45 {
                return ads.clone();
            }
        }
    }
    let fresh = get_adapters();
    if let Ok(mut guard) = AD_CACHE.lock() {
        *guard = Some((std::time::Instant::now(), fresh.clone()));
    }
    fresh
}

/// Énumération 100% native via GetAdaptersAddresses (iphlpapi) : quelques ms,
/// AUCUN spawn powershell. Reprend les mêmes champs que Get-NetAdapter.
/// Retourne None si l'API échoue (le polling bascule sur le repli PowerShell).
pub fn get_adapters_native() -> Option<Vec<Adapter>> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_ALL_INTERFACES, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    use windows::core::{PSTR, PWSTR};

    const NO_ERROR: u32 = 0;
    const ERR_OVERFLOW: u32 = 111; // ERROR_BUFFER_OVERFLOW

    fn pstr_to_string(p: PSTR) -> String {
        if p.is_null() {
            return String::new();
        }
        unsafe {
            std::ffi::CStr::from_ptr(p.0 as *const std::os::raw::c_char)
                .to_string_lossy()
                .into_owned()
        }
    }
    fn pwstr_to_string(p: PWSTR) -> String {
        if p.is_null() {
            return String::new();
        }
        unsafe {
            let mut len = 0usize;
            while *p.0.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p.0, len))
        }
    }

    unsafe {
        let mut size: u32 = 0;
        let rc = GetAdaptersAddresses(2, GAA_FLAG_INCLUDE_ALL_INTERFACES, None, None, &mut size);
        if rc != ERR_OVERFLOW || size == 0 || size > 10_000_000 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let list = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        if GetAdaptersAddresses(2, GAA_FLAG_INCLUDE_ALL_INTERFACES, None, Some(list), &mut size)
            != NO_ERROR
        {
            return None;
        }
        // Parité Get-NetAdapter (sans -IncludeHidden) : on écarte les pilotes
        // de filtre (WFP/QoS), miniports, loopback et adaptateurs virtuels
        // fantômes (hosted network / Wi-Fi Direct) que Get-NetAdapter masque.
        fn visible(name: &str, desc: &str, mac_len: usize) -> bool {
            if name.is_empty() || name == "?" || mac_len == 0 {
                return false;
            }
            let n = name.to_lowercase();
            let d = desc.to_lowercase();
            for pat in [
                "loopback",
                "lightweight filter",
                "filter driver",
                "packet scheduler",
                "miniport",
                "isatap",
                "teredo",
                "6to4",
            ] {
                if n.contains(pat) {
                    return false;
                }
            }
            for pat in ["hosted network", "wi-fi direct virtual", "loopback"] {
                if d.contains(pat) {
                    return false;
                }
            }
            true
        }
        let mut out = vec![];
        let mut cur = list as *const IP_ADAPTER_ADDRESSES_LH;
        while !cur.is_null() {
            let a = &*cur;
            let name = pwstr_to_string(a.FriendlyName);
            let desc = pwstr_to_string(a.Description);
            let mac_len = (a.PhysicalAddressLength as usize).min(8);
            if visible(&name, &desc, mac_len) {
                let mac = a.PhysicalAddress[..mac_len]
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join("-");
                let guid = pstr_to_string(a.AdapterName);
                // IfOperStatusUp = 1 ; seul "Up" compte pour le choix auto.
                let up = a.OperStatus.0 == 1;
                let ifindex = a.Anonymous1.Anonymous.IfIndex as i64;
                // 1ère IPv4 unicast non-loopback, préférence au bail non-APIPA.
                // S_un_b = octets dans l'ordre réseau (S_addr u32 donnerait l'inverse).
                let mut ip = "-".to_string();
                let mut ua = a.FirstUnicastAddress;
                while !ua.is_null() {
                    let e = &*ua;
                    let sa = e.Address.lpSockaddr;
                    if !sa.is_null() && (*sa).sa_family == AF_INET {
                        let sin = &*(sa as *const SOCKADDR_IN);
                        let b = sin.sin_addr.S_un.S_un_b;
                        let v4 = std::net::Ipv4Addr::new(b.s_b1, b.s_b2, b.s_b3, b.s_b4);
                        if !v4.is_loopback() {
                            let s = v4.to_string();
                            if ip == "-" || (ip.starts_with("169.254.") && !s.starts_with("169.254.")) {
                                ip = s;
                            }
                        }
                    }
                    ua = (*ua).Next;
                }
                out.push(Adapter {
                    name,
                    desc,
                    mac,
                    status: if up { "Up".into() } else { "Disconnected".into() },
                    guid,
                    ifindex: Some(ifindex),
                    ip,
                });
            }
            cur = (*cur).Next as *const IP_ADAPTER_ADDRESSES_LH;
        }
        if out.is_empty() { None } else { Some(out) }
    }
}

pub fn get_adapters() -> Vec<Adapter> {
    // Voie rapide : API native (ms, zéro spawn). Replis PowerShell sinon.
    if let Some(native) = get_adapters_native() {
        return native;
    }
    // UN seul powershell (~2s) : les 2 requêtes dans le même script,
    // @() garantit des tableaux JSON même à 0/1 élément.
    let (code, out, _) = run_ps(
        "$adapters = @(Get-NetAdapter | Select-Object Name,InterfaceDescription,MacAddress,Status,InterfaceGuid,IfIndex); \
         $ips = @(Get-NetIPAddress -AddressFamily IPv4 | Select-Object InterfaceIndex,IPAddress); \
         @{ adapters = $adapters; ips = $ips } | ConvertTo-Json -Compress -Depth 3",
    );
    if code == 0 && !out.trim().is_empty() {
        if let Ok(Value::Object(o)) = serde_json::from_str::<Value>(&out) {
            let mut adapters = parse_adapters(o.get("adapters").cloned().unwrap_or(Value::Null));
            if !adapters.is_empty() {
                apply_ipmap(&mut adapters, o.get("ips").cloned().unwrap_or(Value::Null));
                return adapters;
            }
        }
    }
    // Repli : ancienne méthode en 2 appels si le groupé échoue
    get_adapters_split()
}

fn get_adapters_split() -> Vec<Adapter> {
    let (code, out, _) = run_ps(
        "Get-NetAdapter | Select-Object Name,InterfaceDescription,MacAddress,Status,InterfaceGuid,IfIndex | ConvertTo-Json -Compress -Depth 3",
    );
    if code != 0 || out.trim().is_empty() {
        return vec![];
    }
    let parsed: Value = match serde_json::from_str(&out) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut adapters = parse_adapters(parsed);

    let (code2, out2, _) = run_ps(
        "Get-NetIPAddress -AddressFamily IPv4 | Select-Object InterfaceIndex,IPAddress,PrefixOrigin | ConvertTo-Json -Compress -Depth 3",
    );
    if code2 == 0 && !out2.trim().is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(&out2) {
            apply_ipmap(&mut adapters, v);
        }
    }
    adapters
}

fn fetch_ip(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    let t = client
        .get(url)
        .send()
        .map_err(|e| format!("{e}"))?
        .text()
        .map_err(|e| format!("{e}"))?;
    match serde_json::from_str::<Value>(&t) {
        Ok(Value::Object(o)) => o
            .get("ip")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "réponse vide".to_string()),
        _ => Err("réponse inattendue".into()),
    }
}

pub fn get_public_ip() -> String {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0")
        .build()
    {
        Ok(c) => c,
        Err(e) => return format!("ERR: {e}"),
    };
    // HTTPS d'abord, repli HTTP (même fournisseur) si le TLS/DNS coince.
    match fetch_ip(&client, "https://api.ipify.org?format=json") {
        Ok(ip) => ip,
        Err(e1) => match fetch_ip(&client, "http://api.ipify.org?format=json") {
            Ok(ip) => ip,
            Err(e2) => format!("ERR: {e1} / repli: {e2}"),
        },
    }
}

pub fn proxy_reachable(host: &str, port: u16, timeout_secs: u64) -> bool {
    let h = host.trim();
    let addr_str = format!("{h}:{port}");
    let addrs: Vec<SocketAddr> = match dns_lookup(&addr_str) {
        Some(a) => a,
        None => return false,
    };
    for a in addrs {
        if TcpStream::connect_timeout(&a, Duration::from_secs(timeout_secs)).is_ok() {
            return true;
        }
    }
    false
}

fn dns_lookup(addr: &str) -> Option<Vec<SocketAddr>> {
    use std::net::ToSocketAddrs;
    addr.to_socket_addrs().ok().map(|it| it.collect())
}

// ---------- MAC utils ----------

const LOCAL_ADMIN_FIRST: [u8; 32] = [
    0x02, 0x06, 0x0A, 0x0E, 0x12, 0x16, 0x1A, 0x1E, 0x22, 0x26, 0x2A, 0x2E, 0x32, 0x36, 0x3A,
    0x3E, 0x42, 0x46, 0x4A, 0x4E, 0x52, 0x56, 0x5A, 0x5E, 0x62, 0x66, 0x6A, 0x6E, 0x72, 0x76,
    0x7A, 0x7E,
];

const LOCAL_ADMIN_FIRST_B: [u8; 32] = [
    0x82, 0x86, 0x8A, 0x8E, 0x92, 0x96, 0x9A, 0x9E, 0xA2, 0xA6, 0xAA, 0xAE, 0xB2, 0xB6,
    0xBA, 0xBE, 0xC2, 0xC6, 0xCA, 0xCE, 0xD2, 0xD6, 0xDA, 0xDE, 0xE2, 0xE6, 0xEA, 0xEE,
    0xF2, 0xF6, 0xFA, 0xFE,
];

pub fn random_mac_colon() -> String {
    let mut rng = rand::rng();
    let all = [LOCAL_ADMIN_FIRST.as_slice(), LOCAL_ADMIN_FIRST_B.as_slice()].concat();
    let first = all[rng.random_range(0..all.len())];
    let b: Vec<u8> = (0..5).map(|_| rng.random_range(0..=255)).collect();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        first, b[0], b[1], b[2], b[3], b[4]
    )
}

pub fn clean_mac(mac: &str) -> String {
    mac.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase()
}

pub fn dash_mac(nosep: &str) -> String {
    let s = clean_mac(nosep);
    if s.len() != 12 {
        return nosep.to_uppercase();
    }
    (0..12)
        .step_by(2)
        .map(|i| &s[i..i + 2])
        .collect::<Vec<_>>()
        .join("-")
}

pub fn valid_mac_nosep(nosep: &str) -> bool {
    let s = clean_mac(nosep);
    s.len() == 12 && s.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_bypass(s: &str) -> Vec<String> {
    s.replace(';', ",")
        .replace('\n', ",")
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

pub fn pick_target_adapter<'a>(adapters: &'a [Adapter], pref: &str) -> Option<&'a Adapter> {
    if adapters.is_empty() {
        return None;
    }
    if !pref.is_empty() && pref != "Auto" {
        if let Some(a) = adapters.iter().find(|a| a.name == pref) {
            return Some(a);
        }
    }
    if let Some(a) = adapters.iter().find(|a| a.status == "Up") {
        return Some(a);
    }
    adapters.first()
}

// ---------- DHCP ----------

pub fn renew_dhcp(alias: &str) {
    crate::ps::run_quiet("ipconfig", &["/renew", alias]);
}

pub fn current_ipv4(alias: &str) -> String {
    let safe = alias.replace('\'', "''");
    let script = format!(
        "Get-NetIPAddress -AddressFamily IPv4 -InterfaceAlias '{safe}' | Select-Object IPAddress | ConvertTo-Json -Compress -Depth 2"
    );
    let (code, out, _) = run_ps(&script);
    if code != 0 || out.trim().is_empty() {
        return "-".into();
    }
    let v: Value = match serde_json::from_str(&out) {
        Ok(v) => v,
        Err(_) => return "-".into(),
    };
    let mut best = "-".to_string();
    for e in as_array(v) {
        let ip = str_field(&e, &["IPAddress"]);
        if ip.is_empty() || ip.starts_with("127.") {
            continue;
        }
        if ip.starts_with("169.254.") {
            if best == "-" {
                best = ip;
            }
        } else {
            return ip;
        }
    }
    best
}

/// Attend un bail DHCP valide (non-APIPA). Retourne l'IP ou 169.254/- .
pub fn wait_for_valid_ip(alias: &str, timeout_secs: u64) -> String {
    std::thread::sleep(Duration::from_secs(3));
    renew_dhcp(alias);
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut last = "-".to_string();
    while std::time::Instant::now() < deadline {
        last = current_ipv4(alias);
        if last != "-" && !last.starts_with("169.254.") {
            return last;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    last
}
