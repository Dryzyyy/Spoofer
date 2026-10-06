use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::paths::{originals_path, settings_path};

pub fn default_settings() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("ptype".into(), Value::String("SOCKS5 (Tor / autre)".into()));
    m.insert("phost".into(), Value::String("127.0.0.1".into()));
    m.insert("pport".into(), Value::String("9050".into()));
    m.insert("ip_method".into(), Value::String("tun_proxy".into()));
    m.insert("wg_path".into(), Value::String(String::new()));
    m.insert("mac_iface".into(), Value::String("Auto".into()));
    m.insert("custom_mac".into(), Value::String(String::new()));
    m.insert("bypass_apps".into(), Value::String("Discord.exe".into()));
    m
}

fn get_str(s: &Map<String, Value>, k: &str) -> String {
    s.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn s_str(s: &Map<String, Value>, k: &str) -> String {
    get_str(s, k)
}

pub fn load_settings() -> Map<String, Value> {
    let mut s = default_settings();
    if let Ok(p) = settings_path() {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&txt) {
                for (k, v) in obj {
                    s.insert(k, v);
                }
            }
        }
    }
    s
}

pub fn save_settings(s: &Map<String, Value>) -> Result<(), String> {
    let p = settings_path()?;
    let txt = serde_json::to_string_pretty(&Value::Object(s.clone()))
        .map_err(|e| e.to_string())?;
    std::fs::write(&p, txt).map_err(|e| format!("écriture settings.json: {e}"))?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Original {
    pub real: String,
    pub guid: String,
}

pub fn load_originals() -> HashMap<String, Original> {
    let mut out = HashMap::new();
    if let Ok(p) = originals_path() {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(&txt) {
                for (k, v) in obj {
                    out.insert(
                        k,
                        Original {
                            real: v.get("real").and_then(|x| x.as_str()).unwrap_or("").to_uppercase(),
                            guid: v.get("guid").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        },
                    );
                }
            }
        }
    }
    out
}

pub fn store_originals(map: &HashMap<String, Original>) -> Result<(), String> {
    let p = originals_path()?;
    let mut obj = Map::new();
    for (k, o) in map {
        obj.insert(
            k.clone(),
            serde_json::json!({"real": o.real, "guid": o.guid}),
        );
    }
    let txt = serde_json::to_string_pretty(&Value::Object(obj)).map_err(|e| e.to_string())?;
    std::fs::write(&p, txt).map_err(|e| format!("écriture originals.json: {e}"))?;
    Ok(())
}
