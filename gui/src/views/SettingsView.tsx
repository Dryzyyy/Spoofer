import { Icon } from "../components/Icon";
import Seg from "../components/Seg";
import type { Ghost } from "../hooks/useGhostnet";
import type { Route } from "../hooks/useRoute";
import type { Settings } from "../hooks/useSettings";
import IpTab from "./IpTab";
import MacTab, { type MacState } from "./MacTab";
import NetTab from "./NetTab";

type Props = {
  route: Route;
  g: Ghost;
  s: Settings;
  saves: number;
  patch: (p: Partial<Settings>) => void;
  mac: MacState;
  setMac: (m: MacState) => void;
};

const TABS = [
  { value: "ip", label: "IP & tunnel", href: "#/ip" },
  { value: "mac", label: "MAC", href: "#/mac" },
  { value: "reseau", label: "Réseau", href: "#/reseau" },
];

export default function SettingsView({ route, g, s, saves, patch, mac, setMac }: Props) {
  return (
    <section className={"view" + (route !== "main" ? " active" : "")} aria-labelledby="t-set">
      <header className="page-head">
        <div>
          <h1 id="t-set">Réglages</h1>
          <p className={"saved" + (saves ? " pulse" : "")}><Icon key={saves} n="check" />Enregistrés automatiquement dans settings.json</p>
        </div>
      </header>
      <Seg as="nav" kind="link" className="tabs" label="Onglets de réglages" options={TABS} value={route === "main" ? null : route} />
      <IpTab active={route === "ip"} g={g} s={s} patch={patch} />
      <MacTab active={route === "mac"} g={g} s={s} mac={mac} setMac={setMac} patch={patch} />
      <NetTab active={route === "reseau"} g={g} />
    </section>
  );
}
