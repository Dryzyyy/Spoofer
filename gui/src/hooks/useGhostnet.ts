import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Adapter, type Dashboard, type GhostStatus } from "../lib/tauri";

export function useGhostnet() {
  const [status, setStatus] = useState<GhostStatus | null>(null);
  const [adapters, setAdapters] = useState<Adapter[]>([]);
  const [publicIp, setPublicIp] = useState("…");
  const [logs, setLogs] = useState<string[]>(["[GhostNet] GUI démarré."]);
  const [busy, setBusy] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [coreOk, setCoreOk] = useState(true);
  const [bootTimeout, setBootTimeout] = useState(false);
  const busyRef = useRef(false);
  const statusRef = useRef<GhostStatus | null>(null);
  statusRef.current = status;
  // Garde anti-chevauchement pour les refresh manuels.
  const refreshingRef = useRef(false);

  const pushLog = useCallback((line: string) => {
    setLogs((prev) => [...prev.slice(-400), line]);
  }, []);

  const refresh = useCallback(async (withIp = true) => {
    if (refreshingRef.current) return;
    refreshingRef.current = true;
    setRefreshing(true);
    try {
      const d = await api.getDashboard(withIp);
      setCoreOk(true);
      setStatus(d.status);
      setBootTimeout(false);
      setAdapters(d.adapters);
      if (withIp) {
        if (d.publicIp) {
          setPublicIp(d.publicIp);
        } else {
          try {
            setPublicIp(await api.getPublicIp());
          } catch {
            setPublicIp("ERR (Tor chauffe ? Actualiser dans 15s)");
          }
        }
      }
    } catch (e) {
      pushLog(`[ERR] refresh: ${String(e)}`);
    } finally {
      refreshingRef.current = false;
      setRefreshing(false);
    }
  }, [pushLog]);

  useEffect(() => {
    // Watchdog : si RIEN n'arrive en 20s (ni refresh manuel ni event),
    // on l'affiche au lieu de laisser un écran figé muet.
    const w = setTimeout(() => {
      if (!statusRef.current) {
        setBootTimeout(true);
        pushLog("[ERR] backend sans réponse après 20s — voir gui-debug.log");
      }
    }, 20000);
    // État initial (frais + IP), ensuite le thread Rust pousse
    // `ghostnet://state` toutes les 15s : zéro timer côté JS.
    refresh(true);
    const unState = listen<Dashboard>("ghostnet://state", (ev) => {
      setCoreOk(true);
      setStatus(ev.payload.status);
      setBootTimeout(false);
      setAdapters(ev.payload.adapters);
    });
    const unlisten = listen<string>("ghostnet://log", (ev) => {
      pushLog(`[core] ${ev.payload}`);
    });
    return () => {
      clearTimeout(w);
      unState.then((f) => f());
      unlisten.then((f) => f());
    };
  }, [refresh, pushLog]);

  const runGuard = async <T,>(fn: () => Promise<T>): Promise<T | null> => {
    if (busyRef.current) return null;
    busyRef.current = true;
    setBusy(true);
    try {
      return await fn();
    } finally {
      busyRef.current = false;
      setBusy(false);
      await refresh(true);
    }
  };

  const toggle = () =>
    runGuard(async () => {
      const masked = status?.masked;
      pushLog(masked ? "[GUI] coupure demandée…" : "[GUI] masquage demandé… (MAC ~5s + TUN/Tor)");
      try {
        if (masked) {
          await api.maskOff();
          pushLog("[GUI] masquage coupé.");
        } else {
          await api.maskOn();
          pushLog("[GUI] masquage activé.");
        }
      } catch (e) {
        pushLog(`[ERR] ${String(e)}`);
      }
    });

  return { status, adapters, publicIp, logs, busy, refreshing, coreOk, bootTimeout, refresh, toggle, pushLog, runGuard, setPublicIp };
}
