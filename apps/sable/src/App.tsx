import { applyB4ThemeSettings, type B4Theme } from "@b4/tokens";
import { useEffect } from "react";
import { useSession } from "./api";
import { Chrome } from "./Chrome";
import { NewTab } from "./NewTab";
import { Overlay } from "./Overlay";

export function App() {
  const surface = window.__SABLE_SURFACE__ ?? "chrome";
  const { session, error } = useSession();

  useEffect(() => {
    document.documentElement.classList.toggle("surface-overlay", surface === "overlay");
    if (surface === "newtab") return;
    const theme: B4Theme = session?.theme === "porcelain" ? "porcelain" : "graphite";
    applyB4ThemeSettings(document.documentElement, {
      theme,
      density: "compact",
      radius: "default",
      contrast: "default",
      motion: "system",
    });
  }, [session?.theme, surface]);

  if (surface === "newtab") return <NewTab />;
  if (error) return <p className="boot-error">{error}</p>;
  if (!session) return <p className="boot-error">Starting</p>;
  if (surface === "overlay") return <Overlay session={session} />;
  return <Chrome session={session} />;
}
