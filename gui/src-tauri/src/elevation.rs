use std::sync::OnceLock;

static CACHED: OnceLock<bool> = OnceLock::new();

/// Admin ou non ? Mis en cache : l'élévation d'un processus ne change jamais
/// pendant sa vie (relance = nouveau processus). Appel instantané, aucun spawn.
pub fn is_admin() -> bool {
    *CACHED.get_or_init(|| unsafe {
        windows::Win32::UI::Shell::IsUserAnAdmin().as_bool()
    })
}
