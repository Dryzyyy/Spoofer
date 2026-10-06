export type ToastKind = "ok" | "error";
export type ToastItem = { id: number; msg: string; kind: ToastKind; ms: number };

let items: ToastItem[] = [];
let uid = 0;
const subs = new Set<() => void>();
const emit = () => subs.forEach((f) => f());

export function toast(msg: string, kind: ToastKind = "ok", ms = 3400) {
  items = [...items, { id: ++uid, msg, kind, ms }].slice(-4);
  emit();
}
export function dismiss(id: number) {
  items = items.filter((t) => t.id !== id);
  emit();
}
export const subscribeToasts = (f: () => void) => {
  subs.add(f);
  return () => void subs.delete(f);
};
export const getToasts = () => items;
