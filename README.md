# GhostNet — Masque IP + MAC (Tauri + React, backend 100 % Rust)

Interrupteur unique **ACTIVER / COUPER LE MASQUAGE** : proxy système, tunnel
TUN (sing-box → proxy ou WireGuard), Tor intégré auto-téléchargé, spoof MAC
via registre + renouvellement DHCP. Un seul exe portable, aucun Python.

## Lancement

```bat
D:\Spoofer\run-admin.bat
```

Sans admin : proxy seul + affichage. MAC + TUN exigent admin (bouton
« Relancer admin » intégré). Réglages persistés dans `settings.json`, origine
MAC dans `originals.json`.

> **Windows Defender** : l'exe non signé qui touche registre/réseau/processus
> est parfois classé `Trojan:Win32/Wacatac.B!ml` (faux positif) et mis en
> quarantaine → le `.bat` l'explique. Exclusion en place sur `D:\Spoofer`.
> Seule une signature de code ferait disparaître l'alerte définitivement.

## Dossier

```
D:\Spoofer\
  GhostNet-GUI.exe   <- tout-en-un (frontend React + backend Rust)
  run-admin.bat      <- lance en admin
  settings.json      <- réglages par défaut (ptype/phost/pport/méthode/MAC/bypass)
  bin\               <- sing-box.exe, wintun.dll, tor\ (hors git, auto-téléchargés)
  gui\               <- sources (Vite React-TS + src-tauri Rust)
```

`originals.json` (vraies MAC), `sing-tun.json` (config générée) et les logs
restent locaux (hors git).

## Backend Rust (`gui/src-tauri/src/`, 17 commandes Tauri)

| Module | Rôle |
|---|---|
| `lib.rs` | Commandes + ON/OFF + thread de polling (`ghostnet://state` / 15s) + logs |
| `net.rs` | Cartes via `GetAdaptersAddresses` natif (0 spawn), IP publique, DHCP, MAC aléatoires *locally administered* |
| `proxy.rs` | Proxy HKCU + `InternetSetOptionW` |
| `mac.rs` | `NetworkAddress` via GUID/DriverDesc, `Restart-NetAdapter` |
| `tun.rs` | sing-box/wintun (zip), configs SOCKS/WireGuard, `GhostTUN`, anti-fuite `strict_route` |
| `tor.rs` | Expert-bundle (tar.gz), torrc `127.0.0.1:9050`, bootstrap %, `ensure()` |
| `settings.rs` / `paths.rs` | `settings.json` + `originals.json` à côté de l'exe |

Sécurités conservées : refus TUN si proxy injoignable, APIPA `169.254`
signalée, split-tunneling `Discord.exe`, DHCP 30s, Tor 150s max, DNS DoH
`1.1.1.1`, timeouts anti-freeze partout, polling event-driven (zéro
`setInterval`), `gui-debug.log` pour diagnostiquer.

## Rebuild

```bat
cd gui
cmd /c "npm install && npm run build"
cmd /c vsenv.cmd npx tauri build
copy src-tauri\target\release\gui.exe ..\GhostNet-GUI.exe
```

`vsenv.cmd` charge MSVC + cargo. Tests : `vsenv.cmd cargo test
--manifest-path src-tauri\Cargo.toml` (12 tests : MAC, bypass, proxy,
WireGuard, parité natif/PowerShell, lectures réelles).
