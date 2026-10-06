// Aperçu navigateur (`npm run dev` hors Tauri) : simule le backend Rust avec des données fictives.
// Options d'URL : ?admin=0 (sans droits admin), ?down=1 (backend en panne).
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

const q = new URLSearchParams(location.search);
const admin = q.get("admin") !== "0";
const down = q.get("down") === "1";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const log = (m: string) => emit("ghostnet://log", m);

const nets = [
  { name: "Wi-Fi", desc: "Intel Wi-Fi 6 AX201", mac: "A4-83-E7-2B-19-C0", ip: "192.168.1.24", status: "Up", guid: "{a}", ifindex: 12 },
  { name: "Ethernet", desc: "Realtek PCIe GbE", mac: "D8-5E-D3-1A-90-7F", ip: "192.168.1.31", status: "Up", guid: "{b}", ifindex: 7 },
  { name: "Ethernet 2", desc: "USB Ethernet", mac: "5C-BA-EF-73-08-D1", ip: "169.254.38.12", status: "Disconnected", guid: "{c}", ifindex: 21 },
];
const EXITS = ["203.0.113.42", "203.0.113.87", "192.0.2.118"];
let settings: Record<string, string> = {
  ptype: "SOCKS5 (Tor / autre)", phost: "127.0.0.1", pport: "9050", ip_method: "tun_proxy",
  wg_path: "", mac_iface: "Auto", custom_mac: "", bypass_apps: "Discord.exe",
};
let masked = admin;
let spoof = admin ? "02-5E-C4-91-7A-3D" : null;
let exit = 0;
let tunInstalled = true;

const target = () => nets.find((n) => n.name === settings.mac_iface) ?? nets[0];
function dashboard() {
  const adapters = nets.map((n) => {
    const sp = n === nets[0] && spoof;
    return { ...n, effective: sp || n.mac, origine: n.mac, spoof: sp || null, spoofed: !!sp };
  });
  const t = adapters.find((a) => a.name === target().name)!;
  return {
    status: {
      masked, admin,
      proxy: { enabled: masked && settings.ip_method === "proxy", server: "socks=127.0.0.1:9050" },
      tun: { installed: tunInstalled, running: masked && settings.ip_method !== "proxy" },
      tor: { installed: true, running: masked, socks: masked, bootstrap: 100 },
      settings,
      target: { name: t.name, effective: t.effective, origine: t.origine, spoof: t.spoof, spoofed: t.spoofed, ip: t.ip, status: t.status },
    },
    adapters,
  };
}

mockIPC(
  async (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    if (down && cmd !== "get_settings" && cmd !== "save_settings") throw "backend injoignable";
    switch (cmd) {
      case "get_dashboard": {
        await sleep(500);
        return a.withIp ? { ...dashboard(), publicIp: masked ? EXITS[exit] : "198.51.100.17" } : dashboard();
      }
      case "get_public_ip": return masked ? EXITS[exit] : "198.51.100.17";
      case "get_settings": return settings;
      case "save_settings": settings = a.settings as typeof settings; return "ok";
      case "mask_on": {
        if (settings.ip_method === "tun_wg" && !settings.wg_path) throw "Conf WireGuard introuvable — choisis un .conf dans ⚙.";
        if (settings.pport === "1") throw "Proxy 127.0.0.1:1 injoignable — lance Tor / ton proxy AVANT. Sans ça, le TUN couperait tout le réseau.";
        await log("[Tor] démarrage auto…");
        await sleep(700);
        if (admin) {
          spoof = "02-AB-C1-" + Math.floor(Math.random() * 255).toString(16).padStart(2, "0").toUpperCase() + "-7A-3D";
          await log(`[MAC] Wi-Fi → ${spoof}… (coupure ~5s)`);
          await sleep(700);
          await log("[IP locale] Wi-Fi = 192.168.1.24 ✓");
        } else await log("[MAC/IP] non-admin : MAC + TUN exigent admin. Le proxy seul sera appliqué.");
        await sleep(600);
        exit = (exit + 1) % EXITS.length;
        masked = true;
        await log("[TUN] tout l'OS → 127.0.0.1:9050 (pid 4242) ✓");
        return "ok";
      }
      case "mask_off": {
        await log("[TUN] stoppé.");
        await sleep(600);
        if (spoof) await log("[MAC] Wi-Fi restaurée — renouvellement DHCP…");
        await sleep(700);
        spoof = null; masked = false;
        return "ok";
      }
      case "tun_install": {
        await log("[TUN] Téléchargement sing-box…");
        await sleep(1400);
        await log("[TUN] Téléchargement wintun…");
        await sleep(1400);
        tunInstalled = true;
        await log("[TUN] prêt ✓");
        return "ok";
      }
      case "relaunch_as_admin": return "ok";
      default: throw `commande inconnue : ${cmd}`;
    }
  },
  { shouldMockEvents: true },
);

// Le thread de polling Rust pousse l'état toutes les 15 s.
setInterval(() => emit("ghostnet://state", dashboard()), 15000);
