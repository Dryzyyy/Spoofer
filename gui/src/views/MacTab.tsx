import { Icon } from "../components/Icon";
import Collapse from "../components/Collapse";
import type { Ghost } from "../hooks/useGhostnet";
import type { Settings } from "../hooks/useSettings";
import { MAC_RE, macLocal, normMac } from "../lib/mac";
import { colonMac } from "../lib/tauri";

export type MacState = { mode: "random" | "fixed"; fixed: string };

type Props = {
  active: boolean;
  g: Ghost;
  s: Settings;
  mac: MacState;
  setMac: (m: MacState) => void;
  patch: (p: Partial<Settings>) => void;
};

const MODES = [
  { v: "random", title: "MAC aléatoire à chaque activation", desc: "Une nouvelle adresse est générée chaque fois que vous activez le masquage." },
  { v: "fixed", title: "MAC fixe personnalisée", desc: "La même adresse est appliquée à chaque activation." },
] as const;

function hint(raw: string): [string, string] {
  if (!raw.trim()) return ["", ""];
  const v = normMac(raw);
  if (!MAC_RE.test(v)) return ["bad", "Format attendu : 02:00:00:00:00:00"];
  if (!macLocal(v)) return ["warn", "Le deuxième chiffre du premier octet doit valoir 2, 6, A ou E."];
  return ["ok", "Adresse valide"];
}

export default function MacTab({ active, g, s, mac, setMac, patch }: Props) {
  const [kind, msg] = hint(mac.fixed);
  const origin = g.status?.target ? colonMac(g.status.target.origine) : "…";
  return (
    <div className={"panel" + (active ? " active" : "")}>
      <div className="cols">
        <article className="card">
          <h2>Mode de spoofing</h2>
          {MODES.map((m) => (
            <label key={m.v} className="opt">
              <input type="radio" name="macmode" value={m.v} checked={mac.mode === m.v} onChange={() => setMac({ ...mac, mode: m.v })} />
              <span className="radio" />
              <span className="opt-body"><span className="opt-title">{m.title}</span><span className="opt-desc">{m.desc}</span></span>
            </label>
          ))}
          <Collapse open={mac.mode === "fixed"} pad>
            <div className="field">
              <label htmlFor="macFixedIn">Adresse MAC personnalisée</label>
              <input className="input mono" id="macFixedIn" type="text" placeholder="02:00:00:00:00:00" autoComplete="off" spellCheck={false} maxLength={17} style={{ fontSize: 16 }}
                aria-invalid={kind === "bad" || undefined} value={mac.fixed} onChange={(e) => setMac({ ...mac, fixed: e.target.value })} />
              <div className={"field-msg" + (kind ? " " + kind : "")} role="status">{msg}</div>
            </div>
          </Collapse>
          <div className="note"><Icon n="info" /><span>Les adresses générées sont « localement administrées » : le deuxième chiffre du premier octet vaut 2, 6, A ou E.</span></div>
        </article>

        <div className="col">
          <article className="card">
            <h2>Origine constructeur</h2>
            <div className="vbox"><span className="k">MAC d'origine mémorisée</span><div className="v" style={{ color: "var(--text)", fontSize: 22 }}>{origin}</div></div>
            <ul className="checks">
              <li><Icon n="check" />Enregistrée au premier spoof, dans originals.json.</li>
              <li><Icon n="check" />Restaurée automatiquement quand vous coupez le masquage.</li>
            </ul>
          </article>
          <article className="card mt">
            <h2>DHCP</h2>
            <label className="switch">
              <input type="checkbox" checked={s.dhcp_renew !== "false"} onChange={(e) => patch({ dhcp_renew: String(e.target.checked) })} />
              <span className="track" />Forcer le renouvellement du bail DHCP après un changement de MAC
            </label>
            <div className="sub" style={{ color: "var(--muted-2)" }}>Détection APIPA (169.254.x.x)</div>
            <p className="hint">Si la carte n'obtient aucun bail, par exemple quand la box ne répond pas, GhostNet affiche une alerte dans le journal.</p>
            <div className="note warn" role="status"><Icon n="alert" /><span>Exemple d'alerte : aucun bail DHCP, adresse APIPA 169.254.38.12 sur Ethernet 2.</span></div>
          </article>
        </div>
      </div>
    </div>
  );
}
