import { applyB4ThemeSettings, type B4Theme } from "@b4/tokens";
import { useEffect } from "react";

export function NewTab() {
  useEffect(() => {
    const apply = () => {
      const theme: B4Theme = window.__SABLE_THEME__ === "porcelain" ? "porcelain" : "graphite";
      applyB4ThemeSettings(document.documentElement, {
        theme,
        density: "compact",
        radius: "default",
        contrast: "default",
        motion: "system",
      });
    };
    document.title = "New tab";
    apply();
    window.addEventListener("sable-theme", apply);
    return () => window.removeEventListener("sable-theme", apply);
  }, []);

  return (
    <main className="newtab">
      <p className="newtab-mark">Sable</p>
      <p className="newtab-line">Type an address, or a few words.</p>
    </main>
  );
}
