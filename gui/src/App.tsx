import { useEffect, useState } from "react";
import { Icon, Sprite } from "./components/Icon";
import Nav from "./components/Nav";
import Toasts from "./components/Toasts";
import { useGhostnet } from "./hooks/useGhostnet";
import { useRoute, type Route } from "./hooks/useRoute";
import { useSettings, type Settings } from "./hooks/useSettings";
import { isMac } from "./lib/mac";
import MainView from "./views/MainView";
import type { MacState } from "./views/MacTab";
import SettingsView from "./views/SettingsView";

const TITLES: Record<Route, string> = { main: "Principal", ip: "IP & tunnel", mac: "MAC", reseau: "Réseau" };

type ShellProps = {
  s: Settings;
  saves: number;
  patch: (p: Partial<Settings>) => void;
  flush: () => Promise<void>;
};

function Shell({ s, saves, patch, flush }: ShellProps) {
  const g = useGhostnet();
  const route = useRoute();
  const [mac, setMacState] = useState<MacState>(() => ({ mode: s.custom_mac?.trim() ? "fixed" : "random", fixed: s.custom_mac ?? "" }));

  // Le backend choisit « fixe » dès que custom_mac n'est pas vide : en mode aléatoire on l'efface.
  const setMac = (m: MacState) => {
    setMacState(m);
    patch({ custom_mac: m.mode === "fixed" ? m.fixed.trim() : "" });
  };

  const masked = g.status?.masked ?? false;
  useEffect(() => {
    document.body.classList.toggle("is-off", !masked);
  }, [masked]);
  useEffect(() => {
    document.title = TITLES[route] + " – GhostNet";
  }, [route]);

  // Ondulation au clic sur les boutons
  useEffect(() => {
    const on = (e: PointerEvent) => {
      const b = (e.target as Element).closest<HTMLButtonElement>(".btn");
      if (!b || b.disabled || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      const r = b.getBoundingClientRect(), d = Math.max(r.width, r.height) * 2;
      const el = document.createElement("span");
      el.className = "ripple";
      el.style.cssText = `width:${d}px;height:${d}px;left:${e.clientX - r.left - d / 2}px;top:${e.clientY - r.top - d / 2}px`;
      b.append(el);
      el.addEventListener("animationend", () => el.remove());
    };
    document.addEventListener("pointerdown", on);
    return () => document.removeEventListener("pointerdown", on);
  }, []);

  return (
    <div className="app">
      <aside className="rail">
        <a className="brand" href="#/main">
          <span className="brand-mark"><Icon n="ghost" /></span>
          <span>GhostNet</span>
        </a>
        <Nav view={route === "main" ? "main" : "settings"} />
      </aside>
      <div className="stage">
        <div className="content">
          <MainView active={route === "main"} g={g} s={s} patch={patch} flush={flush} macInvalid={mac.mode === "fixed" && !isMac(mac.fixed)} />
          <SettingsView route={route} g={g} s={s} saves={saves} patch={patch} mac={mac} setMac={setMac} />
        </div>
      </div>
    </div>
  );
}

export default function App() {
  const { s, saves, patch, flush } = useSettings();
  return (
    <>
      <div className="bg" aria-hidden="true">
        <div className="bg-layer bg-on" />
        <div className="bg-layer bg-off" />
        <div className="bg-grain" />
      </div>
      <Sprite />
      {s && <Shell s={s} saves={saves} patch={patch} flush={flush} />}
      <Toasts />
    </>
  );
}
