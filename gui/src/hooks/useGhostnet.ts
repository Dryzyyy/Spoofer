import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Adapter, type Dashboard, type GhostStatus } from "../lib/tauri";
import { toast } from "../lib/toast";

export type LogLevel = "info" | "warn" | "error";
export type LogEntry = { id: number; ts: number; t: string; l: LogLevel; m: string };

const IP_UNAVAILABLE = "Indisponible";

let uid = 0;
const entry = (m: string, l: LogLevel): LogEntry => ({
  id: ++uid,
  ts: Date.now(),
  t: new Date().toLocaleTimeString("fr-FR"),
  l,
  m,
});

// Le backend émet des lignes de texte libre ("[MAC ERR …] …", "[IP locale] ⚠ …") : on en déduit le niveau.
const classify = (m: string): LogLevel =>
  /\bERR\b|erreur|échec|injoignable|invalide|introuvable|pas prêt|n'a pas démarré/i.test(m) ? "error"
  : /⚠|APIPA|non-admin|impossible|sautée/i.test(m) ? "warn"
  : "info";

export function useGhostnet() {
  const [status, setStatus] = useState<GhostStatus | null>(null);
  const [adapters, setAdapters] = useState<Adapter[]>([]);
  const [publicIp, setPublicIp] = useState("…");
  const [logs, setLogs] = useState<LogEntry[]>(() => [entry("GhostNet démarré", "info")]);
  const [busy, setBusy] = useState<"on" | "off" | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [backendDown, setBackendDown] = useState(false);
  const [tick, setTick] = useState(0);
  const busyRef = useRef(false);
  const refreshingRef = useRef(false);
  const statusRef = useRef<GhostStatus | null>(null);
  statusRef.current = status;

  const pushLog = useCallback((m: string, l?: LogLevel) => {
    setLogs((prev) => [entry(m, l ?? classify(m)), ...prev].slice(0, 200));
  }, []);

  const refresh = useCallback(
    async (withIp = true): Promise<boolean> => {
      if (refreshingRef.current) return false;
      refreshingRef.current = true;
      setRefreshing(true);
      try {
        const d = await api.getDashboard(withIp);
        setBackendDown(false);
        setStatus(d.status);
        setAdapters(d.adapters);
        if (withIp) {
          // get_public_ip renvoie "ERR: …" en cas d'échec au lieu d'une erreur.
          const ip = d.publicIp ?? (await api.getPublicIp().catch((e) => `ERR: ${e}`));
          if (ip.startsWith("ERR")) {
            setPublicIp(IP_UNAVAILABLE);
            pushLog(`IP publique indisponible : ${ip.replace(/^ERR:?\s*/, "")}`, "warn");
          } else {
            setPublicIp(ip);
            pushLog(`IP publique actualisée : ${ip}`, "info");
          }
        }
        return true;
      } catch (e) {
        setBackendDown(true);
        pushLog(`Actualisation impossible : ${e}`, "error");
        return false;
      } finally {
        refreshingRef.current = false;
        setRefreshing(false);
      }
    },
    [pushLog],
  );

  useEffect(() => {
    // Watchdog : si rien n'arrive en 20 s, on le dit au lieu de laisser un écran figé.
    const w = setTimeout(() => {
      if (!statusRef.current) {
        setBackendDown(true);
        pushLog("Le backend ne répond pas après 20 s : voir gui-debug.log", "error");
      }
    }, 20000);
    // État initial (frais + IP), ensuite le thread Rust pousse `ghostnet://state` toutes les 15 s.
    refresh(true);
    const unState = listen<Dashboard>("ghostnet://state", (ev) => {
      setBackendDown(false);
      setStatus(ev.payload.status);
      setAdapters(ev.payload.adapters);
      setTick((n) => n + 1);
    });
    const unLog = listen<string>("ghostnet://log", (ev) => pushLog(ev.payload));
    return () => {
      clearTimeout(w);
      unState.then((f) => f());
      unLog.then((f) => f());
    };
  }, [refresh, pushLog]);

  /** Bascule le masquage. Renvoie false si l'opération a échoué (le message est déjà affiché). */
  const toggle = useCallback(async (): Promise<boolean> => {
    const cur = statusRef.current;
    if (busyRef.current || !cur) return false;
    const turnOn = !cur.masked;
    busyRef.current = true;
    setBusy(turnOn ? "on" : "off");
    pushLog(turnOn ? "Activation du masquage…" : "Coupure du masquage…", "info");
    let ok = true;
    try {
      await (turnOn ? api.maskOn() : api.maskOff());
      pushLog(turnOn ? "Masquage activé" : "Masquage coupé", "info");
      toast(turnOn ? "Masquage activé" : "Masquage coupé");
    } catch (e) {
      ok = false;
      pushLog(String(e), "error");
      toast(String(e), "error", 6000);
    } finally {
      // Le statut est relu avant de rendre la main : l'interface ne repasse pas par l'ancien état.
      while (refreshingRef.current) await new Promise((r) => setTimeout(r, 100));
      await refresh(true);
      busyRef.current = false;
      setBusy(null);
    }
    return ok;
  }, [refresh, pushLog]);

  return { status, adapters, publicIp, logs, busy, refreshing, backendDown, tick, refresh, toggle, pushLog };
}

export type Ghost = ReturnType<typeof useGhostnet>;
