import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Icon } from "../components/Icon";
import Collapse from "../components/Collapse";
import Seg from "../components/Seg";
import type { Ghost } from "../hooks/useGhostnet";
import type { Settings } from "../hooks/useSettings";
import { shake } from "../lib/dom";
import { api, methodLabel } from "../lib/tauri";
import { toast } from "../lib/toast";

type Props = { active: boolean; g: Ghost; s: Settings; patch: (p: Partial<Settings>) => void };

const METHODS = [
  { v: "proxy", title: "Proxy seul", desc: "Seules les applications qui respectent le proxy passent par lui, comme les navigateurs." },
  { v: "tun_proxy", title: "TUN → proxy", badge: true, desc: "Tout l'OS passe par un tunnel TUN relayé vers votre proxy HTTP ou SOCKS5." },
  { v: "tun_wg", title: "TUN → WireGuard (.conf)", desc: "Full-VPN : tout l'OS passe par votre serveur WireGuard (Mullvad, ProtonVPN, VPS perso)." },
];
const PTYPES = [{ value: "http", label: "HTTP" }, { value: "socks5", label: "SOCKS5" }];
const SOCKS = "SOCKS5 (Tor / autre)";

const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;
const parseApps = (v = "") => v.split(/[;,\n]/).map((x) => x.trim()).filter(Boolean);

export default function IpTab({ active, g, s, patch }: Props) {
  const method = s.ip_method || "tun_proxy";
  const isSocks = (s.ptype ?? "").includes("SOCKS");
  const wgPath = s.wg_path ?? "";
  const apps = parseApps(s.bypass_apps);
  const wgOpen = method === "tun_wg";

  const [preset, setPreset] = useState(0);
  const [over, setOver] = useState(false);
  const [newApp, setNewApp] = useState<string | null>(null);
  const [leaving, setLeaving] = useState<string[]>([]);
  const [appVal, setAppVal] = useState("");
  const [appBad, setAppBad] = useState(false);
  const [since, setSince] = useState(0);
  const [pct, setPct] = useState(0);
  const [progOpen, setProgOpen] = useState(false);
  const dropRef = useRef<HTMLDivElement>(null);
  const appRef = useRef<HTMLInputElement>(null);
  const nameRef = useRef("");
  if (wgPath) nameRef.current = baseName(wgPath);
  const live = useRef(s);
  live.current = s;

  /* ───── WireGuard ───── */
  const take = (path: string | null | undefined) => {
    if (!path) return;
    if (!/\.conf$/i.test(path)) {
      toast("Fichier refusé : choisissez un fichier .conf WireGuard.", "error", 4400);
      shake(dropRef.current);
      return;
    }
    patch({ wg_path: path });
    toast("Configuration WireGuard sélectionnée");
  };
  const pick = async () => {
    try {
      const f = await open({ filters: [{ name: "WireGuard", extensions: ["conf"] }] });
      if (typeof f === "string") take(f);
    } catch (e) {
      toast(String(e), "error", 4400);
    }
  };

  // Tauri intercepte le glisser-déposer natif : les chemins arrivent par cet événement, pas par le DOM.
  const dnd = useRef({ take, ok: false });
  dnd.current = { take, ok: wgOpen && active };
  useEffect(() => {
    let un: (() => void) | undefined;
    let dead = false;
    try {
      getCurrentWebview()
        .onDragDropEvent((e) => {
          const p = e.payload;
          if (p.type === "leave") return setOver(false);
          const r = dropRef.current?.getBoundingClientRect();
          const k = window.devicePixelRatio || 1;
          const x = p.position.x / k, y = p.position.y / k;
          const inside = !!r && dnd.current.ok && x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
          if (p.type === "drop") {
            setOver(false);
            if (inside) dnd.current.take(p.paths[0]);
          } else setOver(inside);
        })
        .then((f) => (dead ? f() : (un = f)))
        .catch(() => {});
    } catch {
      /* hors Tauri (aperçu navigateur) : pas de glisser-déposer natif */
    }
    return () => {
      dead = true;
      un?.();
    };
  }, []);

  /* ───── Split-tunneling ───── */
  const addApp = () => {
    const v = appVal.trim();
    if (!v || /[;,\n]/.test(v) || apps.some((x) => x.toLowerCase() === v.toLowerCase())) {
      setAppBad(true);
      shake(appRef.current);
      return;
    }
    setAppBad(false);
    patch({ bypass_apps: [...apps, v].join(", ") });
    setNewApp(v);
    setAppVal("");
    toast("Exclue du tunnel : " + v);
  };
  const removeApp = (n: string) => {
    setLeaving((l) => [...l, n]);
    setTimeout(() => {
      patch({ bypass_apps: parseApps(live.current.bypass_apps).filter((x) => x !== n).join(", ") });
      setLeaving((l) => l.filter((x) => x !== n));
    }, 300);
  };

  /* ───── Moteur TUN : la progression suit les étapes annoncées par le backend ───── */
  const installing = since > 0;
  useEffect(() => {
    if (!since) return;
    const seen = (re: RegExp) => g.logs.some((l) => l.ts >= since && re.test(l.m));
    setPct(seen(/wintun/i) ? 65 : seen(/sing-box/i) ? 30 : 8);
  }, [g.logs, since]);
  const install = async () => {
    setPct(8);
    setSince(Date.now());
    setProgOpen(true);
    try {
      if ((await api.tunInstall()) === "ok") {
        setPct(100);
        toast("Moteur TUN installé");
      } else toast("Échec de l'installation du moteur TUN", "error", 5000);
    } catch (e) {
      toast(String(e), "error", 5000);
    }
    setSince(0);
    g.refresh(false);
    setTimeout(() => setProgOpen(false), 900);
  };
  const tunOk = !!g.status?.tun.installed;
  const [engTxt, engCls] = !g.status ? ["…", ""] : installing ? ["Mise à jour", "warn"] : tunOk ? ["Installé", "cyan"] : ["Absent", "warn"];

  /* ───── Tor intégré ───── */
  const tor = g.status?.tor;
  const torPct = tor?.running ? Math.min(100, tor.bootstrap ?? 0) : 0;
  const [torTxt, torCls] = !tor ? ["…", ""] : !tor.running ? ["Arrêté", ""] : torPct >= 100 && tor.socks ? ["Connecté", "cyan"] : ["Démarrage…", "warn"];

  const onMethod = (v: string) => {
    patch({ ip_method: v });
    if (g.status?.masked) g.pushLog(`Méthode : ${methodLabel(v)} (appliquée à la prochaine activation)`, "info");
  };
  const flash = preset ? " flash-in" : "";

  return (
    <div className={"panel" + (active ? " active" : "")}>
      <div className="cols">
        <div className="col">
          <article className="card">
            <h2>Méthode de masquage de l'IP</h2>
            {METHODS.map((m) => (
              <label key={m.v} className="opt">
                <input type="radio" name="method" value={m.v} checked={method === m.v} onChange={() => onMethod(m.v)} />
                <span className="radio" />
                <span className="opt-body">
                  <span className="opt-title">{m.title}{m.badge && <span className="chip solid">Conseillé</span>}</span>
                  <span className="opt-desc">{m.desc}</span>
                </span>
              </label>
            ))}
          </article>

          <Collapse open={!wgOpen} pad>
            <article className="card mt">
              <h2>Proxy</h2>
              <Seg name="ptype" label="Type de proxy" style={{ alignSelf: "flex-start" }} options={PTYPES} value={isSocks ? "socks5" : "http"} onChange={(v) => patch({ ptype: v === "socks5" ? SOCKS : "HTTP" })} />
              <div className="row">
                <div className="field" style={{ flex: "3 1 200px" }}>
                  <label htmlFor="hostIn">Hôte</label>
                  <input key={"h" + preset} className={"input mono" + flash} id="hostIn" type="text" autoComplete="off" spellCheck={false} value={s.phost ?? ""} onChange={(e) => patch({ phost: e.target.value })} />
                </div>
                <div className="field" style={{ flex: "1 1 100px" }}>
                  <label htmlFor="portIn">Port</label>
                  <input key={"p" + preset} className={"input mono" + flash} id="portIn" type="text" inputMode="numeric" autoComplete="off" value={s.pport ?? ""} onChange={(e) => patch({ pport: e.target.value })} />
                </div>
              </div>
              <button className="btn" style={{ alignSelf: "flex-start", color: "var(--cyan)" }} onClick={() => { patch({ ptype: SOCKS, phost: "127.0.0.1", pport: "9050" }); setPreset((n) => n + 1); }}>
                Preset Tor · 127.0.0.1:9050
              </button>
            </article>
          </Collapse>

          <Collapse open={wgOpen} pad>
            <article className="card mt">
              <h2>Configuration WireGuard</h2>
              <div ref={dropRef} className={"drop" + (over ? " over" : "")}>
                <Icon n="upload" />
                <strong>Glissez un fichier .conf WireGuard ici</strong>
                <span className="hint">Fournisseurs compatibles : Mullvad, ProtonVPN, VPS perso</span>
                <button className="btn btn-primary" style={{ minHeight: 48 }} onClick={pick}>Choisir un fichier .conf</button>
              </div>
              <Collapse open={!!wgPath}>
                <div className="filerow" style={{ marginTop: 0 }}>
                  <Icon n="file" />
                  <span className="nm mono" title={wgPath}>{nameRef.current}</span>
                  <button className="icon-btn" aria-label="Retirer le fichier WireGuard" onClick={() => patch({ wg_path: "" })}><Icon n="x" /></button>
                </div>
              </Collapse>
            </article>
          </Collapse>

          <article className="card mt">
            <div>
              <h2>Split-tunneling</h2>
              <p className="hint" style={{ marginTop: 2 }}>Applications exclues du tunnel : elles se connectent en direct.</p>
            </div>
            <ul className="split">
              {apps.length
                ? apps.map((n) => (
                    <li key={n} className={"split-item" + (n === newApp ? " enter" : "") + (leaving.includes(n) ? " leaving" : "")}>
                      <Icon n="app" />
                      <span className="name mono">{n}</span>
                      <button className="icon-btn" aria-label={`Retirer ${n} des exclusions`} onClick={() => removeApp(n)}><Icon n="x" /></button>
                    </li>
                  ))
                : <li className="empty">Aucune application exclue : tout le trafic passe par le tunnel.</li>}
            </ul>
            <div className="note"><Icon n="info" /><span>Utile pour Discord : la voix en UDP et les exits Tor bloqués ne passent pas dans le tunnel.</span></div>
            <div className="row">
              <div className="field grow">
                <label htmlFor="appIn">Ajouter une application</label>
                <input ref={appRef} className="input mono" id="appIn" type="text" placeholder="nom-de-l'application.exe" autoComplete="off" spellCheck={false}
                  aria-invalid={appBad || undefined} value={appVal}
                  onChange={(e) => { setAppVal(e.target.value); setAppBad(false); }}
                  onKeyDown={(e) => e.key === "Enter" && addApp()} />
              </div>
              <button className="btn" style={{ minHeight: 48 }} onClick={addApp}>Ajouter</button>
            </div>
          </article>
        </div>

        <div className="col">
          <article className="card">
            <h2>Moteur TUN</h2>
            {["sing-box", "wintun"].map((n) => (
              <div key={n} className="vbox" style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
                <span className="mono" style={{ fontSize: 15, fontWeight: 500 }}>{n}</span>
                <span className={`chip ${engCls}`}>{engTxt}</span>
              </div>
            ))}
            <p className="hint">Les deux composants sont téléchargés automatiquement, sans rien installer à la main.</p>
            <Collapse open={progOpen}>
              <div className="meter"><span>{`Téléchargement… ${pct} %`}</span></div>
              <div className="progress"><i style={{ width: pct + "%" }} /></div>
            </Collapse>
            <button className="btn" style={{ minHeight: 48 }} disabled={installing} onClick={install}>{tunOk ? "Réinstaller le moteur" : "Installer le moteur"}</button>
          </article>

          <article className="card mt">
            <div className="card-head"><h2>Tor intégré</h2><span key={torTxt} className={`chip pop ${torCls}`}>{torTxt}</span></div>
            <div>
              <div className="meter"><span>Bootstrap</span><span className="mono" style={{ color: "var(--cyan)" }}>{torPct} %</span></div>
              <div className="progress"><i style={{ width: torPct + "%" }} /></div>
            </div>
            <ul className="checks">
              <li><Icon n="check" />Bundle officiel téléchargé automatiquement, une seule fois.</li>
              <li><Icon n="check" />Démarre et s'arrête avec le masquage.</li>
              <li><Icon n="check" />Ne touche jamais à Tor Browser (port 9150).</li>
            </ul>
          </article>

          <article className="card mt">
            <h2>Garde-fous</h2>
            <ul className="checks">
              <li><Icon n="shield" />Le tunnel refuse de démarrer si le proxy est injoignable : pas de coupure réseau.</li>
              <li><Icon n="shield" />Anti-fuite DNS : strict_route et DoH 1.1.1.1.</li>
              <li><Icon n="shield" />Réseau local exclu : la box et l'imprimante restent accessibles.</li>
              <li><Icon n="shield" />Sans admin : repli en proxy seul, avec un message clair.</li>
              <li><Icon n="shield" />Délais d'attente partout contre les gels ; diagnostic dans gui-debug.log.</li>
            </ul>
          </article>
        </div>
      </div>
    </div>
  );
}
