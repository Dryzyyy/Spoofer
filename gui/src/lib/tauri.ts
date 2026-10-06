import { invoke } from "@tauri-apps/api/core";

export type GhostStatus = {
  masked: boolean;
  admin: boolean;
  proxy: { enabled: boolean; server: string };
  tun: { installed: boolean; running: boolean };
  tor: { installed: boolean; running: boolean; socks: boolean; bootstrap: number | null };
  settings: Record<string, string>;
  target: {
    name: string; effective: string; origine: string;
    spoof: string | null; spoofed: boolean; ip: string; status: string;
  } | null;
};

export type Adapter = {
  name: string; desc: string; mac: string; effective: string;
  origine: string; spoof: string | null; spoofed: boolean;
  ip: string; status: string; guid: string; ifindex: number;
};

export type Dashboard = {
  status: GhostStatus;
  adapters: Adapter[];
  publicIp?: string;
};

export const api = {
  isAdmin: () => invoke<boolean>("is_admin"),
  coreExists: () => invoke<boolean>("core_file_exists"),
  getStatus: (includePublicIp = false) =>
    invoke<GhostStatus>("get_status", { includePublicIp }),
  getAdapters: () => invoke<Adapter[]>("get_adapters"),
  getDashboard: (withIp = false) =>
    invoke<Dashboard>("get_dashboard", { withIp }),
  getPublicIp: () => invoke<string>("get_public_ip"),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  saveSettings: (settings: Record<string, string>) =>
    invoke<string>("save_settings", { settings }),
  tunStatus: () => invoke<{ installed: boolean; running: boolean }>("tun_status"),
  tunStop: () => invoke<string>("tun_stop"),
  torStatus: () => invoke<{ installed: boolean; running: boolean; socks: boolean; bootstrap: number | null }>("tor_status"),
  maskOn: () => invoke<string>("mask_on"),
  maskOff: () => invoke<string>("mask_off"),
  tunInstall: () => invoke<string>("tun_install"),
  macApply: (iface: string, random = true) =>
    invoke<string>("mac_apply", { iface, random }),
  macRestore: (iface: string) => invoke<string>("mac_restore", { iface }),
  relaunchAsAdmin: () => invoke<string>("relaunch_as_admin"),
};

export function methodLabel(m?: string) {
  if (m === "proxy") return "proxy seul";
  if (m === "tun_wg") return "TUN → WireGuard";
  return "TUN → proxy";
}
