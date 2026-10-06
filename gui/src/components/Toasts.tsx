import { useEffect, useState, useSyncExternalStore } from "react";
import { dismiss, getToasts, subscribeToasts, type ToastItem } from "../lib/toast";
import { Icon } from "./Icon";

function Toast({ t }: { t: ToastItem }) {
  const [out, setOut] = useState(false);
  const close = () => setOut(true);

  useEffect(() => {
    const id = setTimeout(close, t.ms);
    return () => clearTimeout(id);
  }, [t.ms]);
  useEffect(() => {
    if (!out) return;
    const id = setTimeout(() => dismiss(t.id), 360);
    return () => clearTimeout(id);
  }, [out, t.id]);

  return (
    <div className={`toast ${t.kind}${out ? " out" : ""}`} role={t.kind === "error" ? "alert" : "status"} style={{ "--dur": t.ms + "ms" } as React.CSSProperties}>
      <Icon n={t.kind === "error" ? "circle-alert" : "check"} />
      <span className="msg">{t.msg}</span>
      <button className="icon-btn" aria-label="Fermer la notification" onClick={close}><Icon n="x" /></button>
      <i className="bar" />
    </div>
  );
}

export default function Toasts() {
  const items = useSyncExternalStore(subscribeToasts, getToasts);
  return <div className="toasts">{items.map((t) => <Toast key={t.id} t={t} />)}</div>;
}
