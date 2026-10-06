import { Icon } from "./Icon";
import { useThumb } from "./Seg";

export default function Nav({ view }: { view: "main" | "settings" }) {
  const ref = useThumb<HTMLElement>(".nav-thumb", view, true);
  const item = (v: "main" | "settings") => ({
    className: "nav-item" + (view === v ? " active" : ""),
    "aria-current": view === v ? ("page" as const) : undefined,
  });
  return (
    <nav className="nav" ref={ref} aria-label="Navigation principale">
      <span className="nav-thumb" />
      <a {...item("main")} href="#/main"><Icon n="grid" />Principal</a>
      <a {...item("settings")} href="#/ip"><Icon n="sliders" />Réglages</a>
    </nav>
  );
}
