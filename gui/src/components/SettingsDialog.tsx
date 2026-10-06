import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/tauri";
import { Btn, Card, Field, Select, TextInput } from "./ui";

type Props = {
  openFlag: boolean;
  onClose: () => void;
  onSaved: (msg: string) => void;
  adapters: { name: string }[];
};

const TABS = ["IP & tunnel", "MAC", "Réseau"] as const;

export default function SettingsDialog({ openFlag, onClose, onSaved, adapters }: Props) {
  const [tab, setTab] = useState<(typeof TABS)[number]>("IP & tunnel");
  const [s, setS] = useState<Record<string, string>>({});
  const [tunInfo, setTunInfo] = useState("");
  const [netJson, setNetJson] = useState("");

  useEffect(() => {
    if (!openFlag) return;
    (async () => {
      try {
        setS(await api.getSettings());
      } catch {}
      try {
        const t = await api.tunStatus();
        setTunInfo(t.running ? "🟢 TUN actif" : t.installed ? "🟡 moteur installé, TUN coupé" : "🔴 moteur TUN absent");
      } catch {}
      try {
        setNetJson(JSON.stringify(await api.getAdapters(), null, 2));
      } catch {}
    })();
  }, [openFlag ]);

  if (!openFlag) return null;
  const set = (k: string, v: string) => setS((p) => ({ ...p, [k]: v }));

  const save = async () => {
    await api.saveSettings(s);
    onSaved("[réglages] enregistrés ✓");
    onClose();
  };

  const browse = async () => {
    const f = await open({ filters: [{ name: "WireGuard", extensions: ["conf"] }] });
    if (typeof f === "string") set("wg_path", f);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-3" onClick={onClose}>
      <div
        className="flex max-h-[90vh] w-full max-w-[560px] flex-col rounded-2xl border border-white/10 bg-[#0f1115]"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-white/10 px-4 py-3">
          <h2 className="font-bold text-white">Réglages GhostNet</h2>
          <button className="text-gray-400 hover:text-white" onClick={onClose}>✕</button>
        </div>
        <div className="flex gap-1 px-4 pt-3">
          {TABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`rounded-lg px-3 py-1.5 text-sm font-semibold ${tab === t ? "bg-sky-600 text-white" : "bg-white/5 text-gray-300 hover:bg-white/10"}`}
            >
              {t}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto px-4 py-3">
          {tab === "IP & tunnel" && (
            <div className="space-y-3">
              <Card>
                <p className="mb-2 text-sm font-bold">Méthode de masquage IP</p>
                {(["proxy", "tun_proxy", "tun_wg"] as const).map((v) => (
                  <label key={v} className="flex cursor-pointer items-center gap-2 py-1 text-sm">
                    <input type="radio" checked={s.ip_method === v} onChange={() => set("ip_method", v)} />
                    {v === "proxy" ? "Proxy seul (navigateurs uniquement)"
                      : v === "tun_proxy" ? "TUN → mon proxy (TOUT l'OS) — conseillé"
                      : "TUN → WireGuard .conf (TOUT l'OS, full-VPN)"}
                  </label>
                ))}
              </Card>
              <Card>
                <p className="mb-2 text-sm font-bold text-sky-300">Proxy</p>
                <div className="grid grid-cols-[1fr_1fr_70px] gap-2">
                  <Field label="Type">
                    <Select value={s.ptype ?? ""} onChange={(e) => set("ptype", e.target.value)}>
                      <option>HTTP</option>
                      <option>SOCKS5 (Tor / autre)</option>
                    </Select>
                  </Field>
                  <Field label="Hôte">
                    <TextInput value={s.phost ?? ""} onChange={(e) => set("phost", e.target.value)} />
                  </Field>
                  <Field label="Port">
                    <TextInput value={s.pport ?? ""} onChange={(e) => set("pport", e.target.value)} />
                  </Field>
                </div>
                <div className="mt-2 flex items-center gap-2">
                  <Btn variant="ghost" onClick={() => { set("ptype", "SOCKS5 (Tor / autre)"); set("phost", "127.0.0.1"); set("pport", "9050"); }}>
                    Preset Tor
                  </Btn>
                  <span className="text-xs text-gray-500">lance tor.exe avant d'activer</span>
                </div>
              </Card>
              <Card>
                <p className="mb-2 text-sm font-bold text-sky-300">WireGuard</p>
                <div className="flex gap-2">
                  <TextInput value={s.wg_path ?? ""} onChange={(e) => set("wg_path", e.target.value)} placeholder="C:\...\monvpn.conf" />
                  <Btn variant="ghost" onClick={browse}>…</Btn>
                </div>
                <div className="mt-2 flex gap-2">
                  <Btn variant="ghost" onClick={async () => { onSaved("[TUN] installation…"); await api.tunInstall(); }}>
                    ⬇ Installer moteur TUN
                  </Btn>
                  <Btn variant="ghost" onClick={async () => { await api.tunStop(); onSaved("[TUN] stoppé."); }}>
                    ■ Stop TUN
                  </Btn>
                </div>
                <p className="mt-1 text-xs text-gray-400">{tunInfo}</p>
              </Card>
              <Card>
                <p className="mb-1 text-sm font-bold text-sky-300">Apps exclues du tunnel</p>
                <TextInput value={s.bypass_apps ?? ""} onChange={(e) => set("bypass_apps", e.target.value)} placeholder="Discord.exe" />
                <p className="mt-1 text-xs text-gray-500">Ex : Discord.exe (voix UDP impossible via Tor, exits Tor bloquées par Discord).</p>
              </Card>
            </div>
          )}

          {tab === "MAC" && (
            <div className="space-y-3">
              <Card>
                <Field label="Carte à masquer">
                  <Select value={s.mac_iface ?? "Auto"} onChange={(e) => set("mac_iface", e.target.value)}>
                    <option>Auto</option>
                    {adapters.map((a) => <option key={a.name}>{a.name}</option>)}
                  </Select>
                </Field>
                <div className="mt-2">
                  <Field label="MAC fixe (vide = aléatoire à chaque ON)">
                    <div className="flex gap-2">
                      <TextInput value={s.custom_mac ?? ""} onChange={(e) => set("custom_mac", e.target.value)} placeholder="02:11:22:33:44:55" />
                      <Btn variant="ghost" onClick={() => {
                        const h = () => Math.floor(Math.random() * 256).toString(16).padStart(2, "0").toUpperCase();
                        const first = ["02","06","0A","0E","12","1A","22","2A"][Math.floor(Math.random()*8)];
                        set("custom_mac", `${first}:${h()}:${h()}:${h()}:${h()}:${h()}`);
                      }}>🎲</Btn>
                    </div>
                  </Field>
                </div>
                <p className="mt-1 text-xs text-gray-500">Origine constructeur mémorisée au 1er spoof, restaurée au OFF.</p>
              </Card>
            </div>
          )}

          {tab === "Réseau" && (
            <Card>
              <p className="mb-2 text-xs text-gray-400">Effectif = ce que voit le réseau. Origine = constructeur mémorisée.</p>
              <pre className="max-h-[320px] overflow-auto rounded bg-black/40 p-2 text-[11px] text-emerald-200">{netJson || "…"}</pre>
            </Card>
          )}
        </div>

        <div className="flex justify-end gap-2 border-t border-white/10 px-4 py-3">
          <Btn variant="ghost" onClick={onClose}>Fermer</Btn>
          <Btn variant="green" onClick={save}>💾 Enregistrer</Btn>
        </div>
      </div>
    </div>
  );
}
