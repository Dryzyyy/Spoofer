import { useEffect, useState } from "react";
import { reduce } from "../lib/dom";

/** Paragraphe dont le texte s'efface puis se remplace en fondu. */
export default function SwapText({ text, className = "" }: { text: string; className?: string }) {
  const [shown, setShown] = useState(text);
  const [out, setOut] = useState(false);

  useEffect(() => {
    if (text === shown) {
      setOut(false);
      return;
    }
    if (reduce) {
      setShown(text);
      return;
    }
    setOut(true);
    const id = setTimeout(() => {
      setShown(text);
      setOut(false);
    }, 200);
    return () => clearTimeout(id);
  }, [text]);

  return <p className={`${className} swap-text${out ? " fade-out" : ""}`}>{shown}</p>;
}
