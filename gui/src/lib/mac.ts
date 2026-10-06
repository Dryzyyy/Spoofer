export const MAC_RE = /^([0-9A-F]{2}:){5}[0-9A-F]{2}$/;
export const normMac = (v: string) => v.trim().toUpperCase().replace(/-/g, ":");
export const isMac = (v: string) => MAC_RE.test(normMac(v));
/** Bit « localement administré » levé, bit multicast baissé : 2, 6, A ou E en deuxième chiffre. */
export const macLocal = (m: string) => {
  const b = parseInt(m.slice(0, 2), 16);
  return (b & 2) === 2 && (b & 1) === 0;
};
