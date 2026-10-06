import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { reduce } from "../lib/dom";

export const PLACEHOLDER = "…";

type Props = {
  value: string;
  chars: string;
  label?: string;
  accent?: boolean;
  exposed?: boolean;
  plain?: boolean;
  xl?: boolean;
  vStyle?: CSSProperties;
};

/** Boîte de valeur : le texte se « décode » et un reflet balaie la boîte à chaque changement. */
export default function VBox({ value, chars, label, accent, exposed, plain, xl, vStyle }: Props) {
  const [shown, setShown] = useState(value);
  const [flash, setFlash] = useState(false);
  const prev = useRef(value);

  useEffect(() => {
    if (prev.current === value) return;
    const was = prev.current;
    prev.current = value;
    if (was === PLACEHOLDER || reduce) {
      setShown(value);
      return;
    }
    setFlash(true);
    const start = performance.now(), dur = 700, n = value.length;
    let raf = 0;
    const tick = (now: number) => {
      const p = Math.min(1, (now - start) / dur), fixed = Math.floor(p * n);
      let out = "";
      for (let i = 0; i < n; i++) out += i < fixed || value[i] === ":" || value[i] === "." ? value[i] : chars[(Math.random() * chars.length) | 0];
      setShown(p < 1 ? out : value);
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    // rAF est suspendu quand la fenêtre est masquée : la valeur finale ne doit jamais rester périmée.
    const done = setTimeout(() => setShown(value), dur + 50);
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(done);
      setShown(value);
    };
  }, [value, chars]);

  const cls = ["vbox", accent && "accent", exposed && "exposed", plain && "plain", flash && "flash"].filter(Boolean).join(" ");
  return (
    <div className={cls} onAnimationEnd={() => setFlash(false)}>
      {label && <span className="k">{label}</span>}
      <div className={"v" + (xl ? " xl" : "")} style={vStyle}>{shown}</div>
    </div>
  );
}
