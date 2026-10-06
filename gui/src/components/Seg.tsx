import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import type { CSSProperties, ReactNode } from "react";

/** Place une pastille (.seg-thumb / .nav-thumb) sous l'élément `.active` du conteneur. */
export function useThumb<T extends HTMLElement>(thumb: string, dep: unknown, both = false) {
  const ref = useRef<T>(null);
  const place = useCallback(() => {
    const root = ref.current;
    const th = root?.querySelector<HTMLElement>(thumb);
    if (!root || !th) return;
    const a = root.querySelector<HTMLElement>(".active");
    if (!a || !a.offsetWidth) {
      th.style.opacity = "0";
      return;
    }
    th.style.opacity = "1";
    th.style.width = a.offsetWidth + "px";
    if (both) th.style.height = a.offsetHeight + "px";
    th.style.transform = both ? `translate(${a.offsetLeft}px,${a.offsetTop}px)` : `translateX(${a.offsetLeft}px)`;
  }, [thumb, both]);

  useLayoutEffect(place, [place, dep]);
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const ro = new ResizeObserver(place);
    ro.observe(root);
    document.fonts?.ready.then(place);
    return () => ro.disconnect();
  }, [place]);
  return ref;
}

type Opt = { value: string; label: ReactNode; href?: string };
type Props = {
  options: Opt[];
  value: string | null;
  onChange?: (v: string) => void;
  kind?: "radio" | "link" | "static";
  as?: "div" | "nav";
  name?: string;
  label?: string;
  wide?: boolean;
  warn?: boolean;
  className?: string;
  style?: CSSProperties;
};

export default function Seg({ options, value, onChange, kind = "radio", as = "div", name, label, wide, warn, className, style }: Props) {
  const ref = useThumb<HTMLDivElement>(".seg-thumb", value);
  const Tag = as;
  const cls = ["seg", wide && "wide", warn && "warn", className].filter(Boolean).join(" ");
  return (
    <Tag ref={ref} className={cls} style={style} aria-label={label} role={kind === "radio" ? "radiogroup" : undefined}>
      <span className="seg-thumb" />
      {options.map((o) => {
        const c = "seg-item" + (o.value === value ? " active" : "");
        if (kind === "link") return <a key={o.value} className={c} href={o.href}>{o.label}</a>;
        if (kind === "static") return <span key={o.value} className={c}>{o.label}</span>;
        return (
          <label key={o.value} className={c}>
            <input type="radio" name={name} value={o.value} checked={o.value === value} onChange={() => onChange?.(o.value)} />
            {o.label}
          </label>
        );
      })}
    </Tag>
  );
}
