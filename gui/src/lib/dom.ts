export const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;

const SHAKE: Keyframe[] = [
  { transform: "none" },
  { transform: "translateX(-6px)", offset: 0.2 },
  { transform: "translateX(5px)", offset: 0.4 },
  { transform: "translateX(-3px)", offset: 0.6 },
  { transform: "translateX(2px)", offset: 0.8 },
  { transform: "none" },
];

export function shake(el: Element | null) {
  if (!el || reduce) return;
  el.animate(SHAKE, { duration: 400, easing: "cubic-bezier(.22,1,.36,1)" });
}
