//! Journal de diagnostic sur disque (gui-debug.log à côté de l'exe).
//! Best-effort : ne panique jamais, ne bloque jamais le backend.
use std::io::Write;

pub fn dlog(msg: &str) {
    let dir = match crate::paths::spoofer_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    let p = dir.join("gui-debug.log");
    // rotation rudimentaire : >2 Mo → on repart de zéro
    if let Ok(m) = std::fs::metadata(&p) {
        if m.len() > 2_000_000 {
            let _ = std::fs::remove_file(&p);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "[{:?}] {}", std::time::SystemTime::now(), msg);
    }
}
