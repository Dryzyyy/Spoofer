import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

const mount = () =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );

// Aperçu navigateur : hors Tauri, le backend est simulé (jamais inclus dans le build de production).
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) import("./dev/mock").then(mount);
else mount();
