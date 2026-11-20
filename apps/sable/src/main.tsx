import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { applyB4ThemeSettings, type B4Theme } from "@b4/tokens";
import "@b4/tokens/css";
import "@b4/react/styles.css";
import { App } from "./App";
import "./styles.css";

const theme: B4Theme = window.__SABLE_THEME__ === "porcelain" ? "porcelain" : "graphite";
applyB4ThemeSettings(document.documentElement, {
  theme,
  density: "compact",
  radius: "default",
  contrast: "default",
  motion: "system",
});

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
