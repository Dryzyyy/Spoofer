import { useEffect, useState } from "react";

export type Route = "main" | "ip" | "mac" | "reseau";
const ROUTES: Route[] = ["main", "ip", "mac", "reseau"];

const read = (): Route => {
  const h = location.hash.replace(/^#\//, "") as Route;
  return ROUTES.includes(h) ? h : "main";
};

export function useRoute(): Route {
  const [route, setRoute] = useState(read);
  useEffect(() => {
    const on = () => {
      setRoute(read());
      scrollTo({ top: 0 });
    };
    addEventListener("hashchange", on);
    return () => removeEventListener("hashchange", on);
  }, []);
  return route;
}
