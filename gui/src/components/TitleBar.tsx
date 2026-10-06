import { useEffect, useState } from "react";
import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
import { Icon } from "./Icon";

/** Barre de titre personnalisée (la fenêtre n'a plus de décoration Windows). */
export default function TitleBar() {
  const [max, setMax] = useState(false);

  useEffect(() => {
    let un: (() => void) | undefined;
    let dead = false;
    try {
      const w = getCurrentWindow();
      const sync = () => w.isMaximized().then(setMax).catch(() => {});
      sync();
      w.onResized(sync).then((f) => (dead ? f() : (un = f))).catch(() => {});
    } catch {
      /* hors Tauri (aperçu navigateur) */
    }
    return () => {
      dead = true;
      un?.();
    };
  }, []);

  const act = (f: (w: Window) => Promise<void>) => () => {
    try {
      f(getCurrentWindow()).catch(() => {});
    } catch {
      /* hors Tauri */
    }
  };

  return (
    <header className="titlebar" data-tauri-drag-region>
      <button className="winbtn" aria-label="Réduire" onClick={act((w) => w.minimize())}><Icon n="minus" /></button>
      <button className="winbtn" aria-label={max ? "Restaurer" : "Agrandir"} onClick={act((w) => w.toggleMaximize())}><Icon n={max ? "restore" : "square"} /></button>
      <button className="winbtn close" aria-label="Fermer" onClick={act((w) => w.close())}><Icon n="x" /></button>
    </header>
  );
}
