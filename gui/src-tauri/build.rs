fn main() {
    // L'exe demande l'élévation (UAC) à chaque lancement : le spoof MAC et le tunnel TUN exigent admin.
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run build script");
}
