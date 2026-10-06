import { useRef, useState } from "react";
import { Icon } from "../components/Icon";
import Collapse from "../components/Collapse";
import Seg from "../components/Seg";
import SwapText from "../components/SwapText";
import VBox from "../components/VBox";
import type { Ghost, LogLevel } from "../hooks/useGhostnet";
import type { Settings } from "../hooks/useSettings";
import { shake } from "../lib/dom";
import { api, colonMac } from "../lib/tauri";
import { toast } from "../lib/toast";

type Props = {
  active: boolean;
  g: Ghost;
  s: Settings;
  patch: (p: Partial<Settings>) => void;
  flush: () => Promise<void>;
  macInvalid: boolean;
};

const LV: Record<LogLevel, [string, string]> = { info: ["Info", "cyan"], warn: ["Avert.", "warn"], error: ["Erreur", "bad"] };
const FILTERS = [
  { value: "all", label: "Tout" },
  { value: "info", label: "Info" },
  { value: "warn", label: "Avertissements" },
  { value: "error", label: "Erreurs" },
];
const COVERS = [
  { value: "tun", label: "TUN total" },
  { value: "proxy", label: "Proxy navigateurs" },
  { value: "direct", label: "Direct exposé" },
];

export default function MainView({ active, g, s, patch, flush, macInvalid }: Props) {
  const { status, adapters, publicIp, logs, busy, refreshing, backendDown, tick, refresh, toggle, pushLog } = g;
  const [filter, setFilter] = useState("all");
  const [memo, setMemo] = useState(0);
  const [retrying, setRetrying] = useState(false);
  const [relaunching, setRelaunching] = useState(false);
  const powerRef = useRef<HTMLButtonElement>(null);

  const loading = !status && !backendDown;
  const masked = status?.masked ?? false;
  const admin = status?.admin ?? true;
  const target = status?.target ?? null;
  const spoofed = target?.spoofed ?? false;
  const ipMasked = !!status && (status.tun.running || status.proxy.enabled);
  const coverage = !status ? null : status.tun.running ? "tun" : status.proxy.enabled ? "proxy" : "direct";

  const heroSub = !status
    ? backendDown ? "État inconnu : le backend ne répond pas." : "Lecture de l'état en cours…"
    : !masked ? "Votre IP publique et votre MAC d'origine sont visibles sur le réseau."
    : ipMasked && spoofed ? "Votre IP publique et votre MAC d'origine sont masquées."
    : ipMasked && admin ? "Votre IP publique est masquée. Votre MAC d'origine reste visible."
    : ipMasked ? "Votre IP publique est masquée par le proxy. Sans admin, votre MAC reste visible."
    : "Votre MAC est masquée, mais votre IP publique reste visible.";

  const powerV = busy || loading ? "busy" : masked ? "on" : "off";
  const busyLabel = loading ? "Lecture de l'état…" : busy === "off" ? "Arrêt en cours…" : "Activation…";

  const cardNow = s.mac_iface || "Auto";
  const cards = adapters.map((a) => a.name);
  if (cardNow !== "Auto" && !cards.includes(cardNow)) cards.unshift(cardNow);

  const onPower = async () => {
    if (busy || !status) return;
    if (!masked && admin && macInvalid) {
      toast("MAC personnalisée invalide. Corrigez-la dans Réglages › MAC.", "error", 4600);
      pushLog("MAC personnalisée invalide : activation annulée", "error");
      shake(powerRef.current);
      return;
    }
    await flush();
    if (!(await toggle())) shake(powerRef.current);
  };

  const onCard = async (v: string) => {
    patch({ mac_iface: v });
    setMemo((n) => n + 1);
    if (masked) pushLog(`Carte cible : ${v} (appliquée à la prochaine activation)`, "info");
    await flush();
    refresh(false);
  };

  const onRefresh = async () => {
    if (busy || refreshing) return;
    const ok = await refresh(true);
    toast(ok ? "État actualisé" : "Le backend ne répond pas", ok ? "ok" : "error");
  };

  const onRetry = async () => {
    setRetrying(true);
    const ok = await refresh(true);
    setRetrying(false);
    toast(ok ? "Connexion au backend rétablie" : "Le backend ne répond toujours pas", ok ? "ok" : "error");
  };

  const onRelaunch = async () => {
    setRelaunching(true);
    try {
      await api.relaunchAsAdmin();
      toast("Relance demandée : validez la fenêtre UAC, puis fermez cette fenêtre.", "ok", 6000);
    } catch (e) {
      toast(String(e), "error", 5000);
    }
    setRelaunching(false);
  };

  const rows = logs.filter((e) => filter === "all" || e.l === filter);

  return (
    <section className={"view" + (active ? " active" : "")} aria-labelledby="t-main">
      <header className="page-head">
        <div>
          <h1 id="t-main">Masquage réseau</h1>
          <p className="muted">Actualisation automatique toutes les 15 s</p>
        </div>
        <div className="head-actions">
          {status && (admin
            ? <span key="a" className="chip cyan big pop"><Icon n="shield-check" />Administrateur</span>
            : <span key="n" className="chip warn big pop"><Icon n="alert" />Sans admin</span>)}
          {status && !admin && <button className="btn btn-amber" onClick={onRelaunch} disabled={relaunching}>{relaunching ? "Relance…" : "Relancer en admin"}</button>}
          <button className={"btn" + (refreshing ? " spinning" : "")} onClick={onRefresh}>
            <svg className="tick" viewBox="0 0 20 20" aria-hidden="true">
              <circle className="bgc" cx="10" cy="10" r="8" />
              <circle key={tick} className="fg run" cx="10" cy="10" r="8" />
            </svg>
            Actualiser
          </button>
        </div>
      </header>

      <Collapse open={backendDown}>
        <div className="banner bad" role="alert">
          <Icon n="circle-alert" />
          <span>Le backend ne répond pas. Les valeurs affichées peuvent être obsolètes.</span>
          <button className="btn" onClick={onRetry} disabled={retrying}>Réessayer</button>
        </div>
      </Collapse>
      <Collapse open={!!status && !admin}>
        <div className="banner warn" role="status">
          <Icon n="alert" />
          <span>Sans droits administrateur, GhostNet se limite au proxy seul. Le changement de MAC et le tunnel TUN exigent admin.</span>
        </div>
      </Collapse>

      <div className="stack">
        <section className="hero" data-state={masked ? "on" : "off"} data-busy={busy || loading ? "1" : "0"} aria-label="État du masquage">
          <div className="orb" aria-hidden="true">
            <span className="ring" /><span className="ring" /><span className="ring" />
            <div className="core"><Icon n="shield-check" className="on" /><Icon n="shield-off" className="off" /></div>
          </div>
          <div className="hero-text">
            <p className="hero-kicker">État du masquage</p>
            <div className="state" aria-live="polite"><div className="swap"><span data-s="on">PROTÉGÉ</span><span data-s="off">EXPOSÉ</span></div></div>
            <SwapText className="hint" text={heroSub} />
          </div>
          <button ref={powerRef} className="btn btn-primary btn-lg power" data-v={powerV} disabled={!!busy || !status} aria-busy={!!busy || loading} onClick={onPower}>
            <Icon n="power" />
            <span className="spinner" aria-hidden="true" />
            <span className="swap">
              <span data-s="on">COUPER LE MASQUAGE</span>
              <span data-s="off">ACTIVER LE MASQUAGE</span>
              <span data-s="busy">{busyLabel}</span>
            </span>
          </button>
        </section>

        <div className="cols">
          <article className="card" aria-labelledby="t-ip">
            <div className="card-head">
              <span className="tile"><Icon n="globe" /></span>
              <h2 id="t-ip">IP publique</h2>
              <span className="chip live"><i className="dot" />En direct</span>
            </div>
            <VBox xl accent exposed={!!status && !ipMasked} value={publicIp} chars="0123456789" />
            <div className="sub">Mode de couverture</div>
            <Seg wide kind="static" warn={coverage === "direct"} options={COVERS} value={coverage} />
            <a className="link" href="#/ip">Modifier la méthode dans Réglages › IP &amp; tunnel</a>
          </article>

          <article className="card" aria-labelledby="t-mac">
            <div className="card-head">
              <span className="tile"><Icon n="chip" /></span>
              <h2 id="t-mac">MAC de la carte cible</h2>
              {target && <span key={String(spoofed)} className={"chip pop" + (spoofed ? " solid" : "")}>{spoofed ? "Spoofée" : "D'origine"}</span>}
            </div>
            <VBox accent plain={!spoofed} label="MAC effective" chars="0123456789ABCDEF" value={target ? colonMac(target.effective) : "…"} />
            <VBox label="MAC d'origine" chars="0123456789ABCDEF" vStyle={{ fontSize: 20 }} value={target ? colonMac(target.origine) : "…"} />
            <div className="row">
              <div className="field grow">
                <label htmlFor="cardSel">Carte cible</label>
                <select className="input" id="cardSel" value={cardNow} onChange={(e) => onCard(e.target.value)}>
                  <option>Auto</option>
                  {cards.map((n) => <option key={n}>{n}</option>)}
                </select>
              </div>
              <span key={memo} className={"chip big" + (memo ? " pop" : "")}><Icon n="check" style={{ width: 16, height: 16 }} />Choix mémorisé</span>
            </div>
          </article>
        </div>

        <article className="card" aria-labelledby="t-log">
          <div className="card-head" style={{ flexWrap: "wrap" }}>
            <h2 id="t-log">Journal en temps réel</h2>
            <Seg name="logfilter" label="Filtrer le journal" options={FILTERS} value={filter} onChange={setFilter} />
          </div>
          <div className="log" role="log" aria-live="polite" aria-relevant="additions">
            {rows.length
              ? rows.map((e) => (
                  <div key={e.id} className={"log-row" + (Date.now() - e.ts < 800 ? " enter" : "")}>
                    <span className="t mono">{e.t}</span>
                    <span className={`chip ${LV[e.l][1]}`}>{LV[e.l][0]}</span>
                    <span className="m">{e.m}</span>
                  </div>
                ))
              : <div className="empty">Aucune entrée pour ce filtre.</div>}
          </div>
        </article>
      </div>
    </section>
  );
}
