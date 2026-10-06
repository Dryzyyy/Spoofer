import { useEffect, useRef, useState } from "react";
import { Toaster, toast } from "sonner";
import { ShieldCheck, ShieldAlert, RefreshCw, Settings as SettingsIcon } from "lucide-react";
import { useGhostnet } from "./hooks/useGhostnet";
import { api, methodLabel } from "./lib/tauri";
import SettingsDialog from "./components/SettingsDialog";
import { Btn, Card, Select } from "./components/ui";

export default function App() {
  const { status, adapters, publicIp, logs, busy, refreshing, coreOk, bootTimeout, refresh, toggle, pushLog } = useGhostnet();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [iface, setIface] = useState("Auto");
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [logs]);

  useEffect(() => {
    api.getSettings().then((s) => setIface(s.mac_iface ?? "Auto")).catch(() => {});
  }, []);

  const masked = status?.masked ?? false;
  const admin = status?.admin ?? false;
  const tunRunning = status?.tun.running ?? false;
  const proxyOn = status?.proxy.enabled ?? false;

  const modeText = tunRunning
    ? "Tunnel TUN actif — tout l'OS passe dedans"
    : proxyOn
    ? `Proxy actif (${status?.proxy.server}) — navigateurs couverts`
    : "Trafic direct — IP réelle exposée";

  const changeIface = async (v: string) => {
    setIface(v);
    try {
      const s = await api.getSettings();
      await api.saveSettings({ ...s, mac_iface: v });
      pushLog(`[carte] cible MAC : ${v}`);
      refresh(false);
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="mx-auto flex min-h-full w-full max-w-[480px] flex-col gap-3 p-3.5">
      <Toaster theme="dark" position="bottom-center" />
      {/* header */}
      <div className="flex items-center gap-2">
        <h1 className="text-xl font-extrabold text-white">👻 GhostNet</h1>
        <span className={`rounded-full px-2 py-0.5 text-xs font-bold ${admin ? "bg-green-900 text-green-300" : "bg-red-900 text-red-300"}`}>
          {admin ? "🟢 admin" : "🔴 non-admin"}
        </span>
        <div className="flex-1" />
        {!admin && (
          <Btn variant="ghost" className="text-xs" onClick={() => api.relaunchAsAdmin()}>
            Relancer admin
          </Btn>
        )}
        <Btn variant="ghost" onClick={() => setSettingsOpen(true)} title="Réglages">⚙</Btn>
      </div>

      {!coreOk && (
        <Card className="border-red-500/40 bg-red-950/60 text-sm text-red-200">
          ghostnet-core.exe introuvable. Copie le GUI à côté du core (racine portable D:\Spoofer).
        </Card>
      )}

      {bootTimeout && !status && (
        <Card className="border-amber-500/40 bg-amber-950/60 text-sm text-amber-200">
          Backend sans réponse après 20s. Ouvre <b>D:\Spoofer\gui-debug.log</b> et
          envoie-moi les dernières lignes — elles disent quelle étape bloque.
        </Card>
      )}

      {/* état global */}
      <Card className="py-4 text-center">
        {masked ? (
          <div className="flex items-center justify-center gap-2 text-2xl font-extrabold text-green-400">
            <ShieldCheck /> PROTÉGÉ
          </div>
        ) : (
          <div className="flex items-center justify-center gap-2 text-2xl font-extrabold text-red-400">
            <ShieldAlert /> EXPOSÉ
          </div>
        )}
        <p className="mt-1 text-xs text-gray-400">{modeText}</p>
      </Card>

      <Btn
        variant={masked ? "red" : "green"}
        className="w-full py-4 text-base font-extrabold tracking-wide"
        disabled={busy}
        onClick={toggle}
      >
        {busy ? "PATIENCE…" : masked ? "COUPER LE MASQUAGE" : "ACTIVER LE MASQUAGE"}
      </Btn>

      {/* infos */}
      <Card className="space-y-1.5 text-sm">
        <p>IP publique : <b className="text-white">{publicIp}</b></p>
        <p className="text-xs text-gray-400">{modeText}</p>
        <div className="border-t border-white/10 pt-2">
          {status?.target ? (
            <>
              <p>MAC {status.target.name} : <b>{status.target.effective}</b></p>
              <p className="text-xs text-gray-400">
                {status.target.spoofed ? `spoofée ✓ (origine : ${status.target.origine})` : "MAC d'origine ✓"}
              </p>
            </>
          ) : (
            <p className="text-gray-400">MAC : …</p>
          )}
          <div className="mt-2 flex items-center gap-2 text-xs">
            <span className="text-gray-400">Carte à masquer :</span>
            <Select value={iface} onChange={(e) => changeIface(e.target.value)}>
              <option>Auto</option>
              {adapters.map((a) => <option key={a.name} value={a.name}>{a.name}</option>)}
            </Select>
          </div>
        </div>
      </Card>

      <div className="flex items-center gap-2">
        <Btn variant="ghost" className="text-xs" disabled={busy || refreshing} onClick={() => {
          pushLog("[GUI] actualisation…");
          refresh(true).then(() => pushLog("[GUI] actualisé ✓"));
        }}>
          <RefreshCw size={14} className={`mr-1 inline ${refreshing ? "animate-spin" : ""}`} />
          {refreshing ? "…" : "Actualiser"}
        </Btn>
        <div className="flex-1" />
        <span className="text-[11px] text-gray-500">
          méthode : {methodLabel(status?.settings.ip_method)}
          {status?.tor.running ? ` · Tor ${status.tor.bootstrap ?? "?"}%` : ""}
        </span>
      </div>

      {/* journal */}
      <p className="text-[11px] text-gray-500">Journal</p>
      <div ref={logRef} className="log-scroll min-h-[140px] flex-1 overflow-y-auto rounded-xl border border-white/10 bg-[#0b0d11] p-2 font-mono text-[11px] leading-relaxed text-emerald-200">
        {logs.map((l, i) => <div key={i}>{l}</div>)}
      </div>

      <div className="flex items-center justify-between pb-1">
        <span className="text-[11px] text-gray-600">Portable · sidecar ghostnet-core.exe</span>
        <button
          className="flex items-center gap-1 text-[11px] text-gray-500 hover:text-white"
          onClick={() => setSettingsOpen(true)}
        >
          <SettingsIcon size={12} /> IP / MAC / Réseau
        </button>
      </div>

      <SettingsDialog
        openFlag={settingsOpen}
        onClose={() => { setSettingsOpen(false); refresh(false); }}
        onSaved={(m) => { pushLog(m); toast.success(m); refresh(false); }}
        adapters={adapters}
      />
    </div>
  );
}
