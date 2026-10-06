import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../lib/tauri";
import { toast } from "../lib/toast";

export type Settings = Record<string, string>;

/**
 * Réglages persistés dans settings.json. Chaque modification est enregistrée
 * automatiquement (200 ms après la dernière frappe) ; `flush` force l'écriture
 * immédiate, à appeler avant toute commande qui relit le fichier côté Rust.
 */
export function useSettings() {
  const [s, setS] = useState<Settings | null>(null);
  const [saves, setSaves] = useState(0);
  const cur = useRef<Settings>({});
  const dirty = useRef(false);
  const timer = useRef(0);

  useEffect(() => {
    api.getSettings()
      .catch(() => ({}) as Settings)
      .then((v) => {
        cur.current = v;
        setS(v);
      });
  }, []);

  const flush = useCallback(async () => {
    clearTimeout(timer.current);
    if (!dirty.current) return;
    dirty.current = false;
    try {
      await api.saveSettings(cur.current);
    } catch (e) {
      toast(`Enregistrement impossible : ${e}`, "error", 5000);
    }
  }, []);

  const patch = useCallback(
    (p: Partial<Settings>) => {
      cur.current = { ...cur.current, ...p } as Settings;
      setS(cur.current);
      setSaves((n) => n + 1);
      dirty.current = true;
      clearTimeout(timer.current);
      timer.current = window.setTimeout(flush, 200);
    },
    [flush],
  );

  return { s, saves, patch, flush };
}
