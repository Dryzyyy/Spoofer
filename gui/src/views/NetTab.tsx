import { useEffect, useState } from "react";
import { Icon } from "../components/Icon";
import type { Ghost } from "../hooks/useGhostnet";
import { colonMac } from "../lib/tauri";
import { toast } from "../lib/toast";

export default function NetTab({ active, g }: { active: boolean; g: Ghost }) {
  const [pulse, setPulse] = useState(0);
  useEffect(() => {
    if (active) setPulse((n) => n + 1);
  }, [active]);

  const onRefresh = async () => {
    if (g.refreshing) return;
    const ok = await g.refresh(true);
    setPulse((n) => n + 1);
    toast(ok ? "Cartes actualisées" : "Le backend ne répond pas", ok ? "ok" : "error");
  };
  const targetName = g.status?.target?.name;

  return (
    <div className={"panel" + (active ? " active" : "")}>
      <article className="card">
        <div className="card-head">
          <h2>Cartes réseau de la machine</h2>
          <button className={"btn" + (g.refreshing ? " spin-on" : "")} onClick={onRefresh}><Icon n="refresh" />Actualiser</button>
        </div>
        <div className="scroll-x">
          <div className="table">
            <div className="tr th">
              <span className="c1">Interface</span><span className="c2">IP locale</span><span className="c3">MAC effective</span><span className="c4">MAC d'origine</span><span className="c5">Statut</span>
            </div>
            {g.adapters.length
              ? g.adapters.map((a) => {
                  const t = a.name === targetName;
                  const apipa = a.ip.startsWith("169.254.");
                  return (
                    <div key={`${a.name}-${t ? pulse : 0}`} className={"tr" + (t && pulse ? " target" : "")}>
                      <span className="c1" style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, fontWeight: 700 }}>{a.name}{t && <span className="chip">Cible</span>}</span>
                      <span className="c2" style={{ display: "flex", flexDirection: "column", alignItems: "flex-start", gap: 4 }}>
                        <span className="m" style={{ color: apipa ? "#FFD684" : "#D4E3EA" }}>{a.ip === "-" ? "—" : a.ip}</span>
                        {apipa && <span className="chip warn">APIPA : sans bail</span>}
                      </span>
                      <span className="c3 m" style={{ color: a.spoofed ? "var(--cyan)" : "#D4E3EA" }}>{colonMac(a.effective)}</span>
                      <span className="c4 m" style={{ color: "var(--muted)" }}>{colonMac(a.origine)}</span>
                      <span className="c5"><span className={"chip" + (a.spoofed ? " solid" : "")}>{a.spoofed ? "Spoofée" : "D'origine"}</span></span>
                    </div>
                  );
                })
              : <div className="empty">Aucune carte réseau détectée.</div>}
          </div>
        </div>
      </article>
    </div>
  );
}
