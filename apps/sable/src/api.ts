import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { SessionDto } from "./types";

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export function useSession(): { session: SessionDto | null; error: string | null } {
  const [session, setSession] = useState<SessionDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const surface = window.__SABLE_SURFACE__;
    if (surface !== "chrome" && surface !== "overlay") return;
    let stop = false;
    let unlisten: UnlistenFn | undefined;
    void (async () => {
      unlisten = await listen<SessionDto>("session", (event) => {
        if (!stop) setSession(event.payload);
      });
      for (let attempt = 0; attempt < 50 && !stop; attempt += 1) {
        try {
          const snapshot = await call<SessionDto>("session_snapshot");
          if (!stop) setSession(snapshot);
          return;
        } catch {
          await new Promise((resolve) => window.setTimeout(resolve, 100));
        }
      }
      if (!stop) setError("The browser core did not answer.");
    })();
    return () => {
      stop = true;
      unlisten?.();
    };
  }, []);

  return { session, error };
}

export const browser = {
  navigate: (input: string) => call<SessionDto>("navigate", { input }),
  back: () => call<SessionDto>("go_back"),
  forward: () => call<SessionDto>("go_forward"),
  reload: () => call<SessionDto>("reload"),
  newTab: () => call<SessionDto>("new_tab"),
  closeTab: (tabId?: string) => call<SessionDto>("close_tab", { tabId }),
  selectTab: (tabId: string) => call<SessionDto>("select_tab", { tabId }),
  pinTab: (tabId: string, pinned: boolean) => call<SessionDto>("pin_tab", { tabId, pinned }),
  moveTab: (tabId: string, toIndex: number) => call<SessionDto>("move_tab", { tabId, toIndex }),
  setLayout: (layout: string) => call<SessionDto>("set_layout", { layout }),
  setTheme: (theme: string) => call<SessionDto>("set_theme", { theme }),
  selectSpace: (spaceId: string) => call<SessionDto>("select_space", { spaceId }),
  newSpace: () => call<SessionDto>("new_space"),
  toggleSplit: () => call<SessionDto>("toggle_split"),
  setSplitRatio: (ratio: number) => call<SessionDto>("set_split_ratio", { ratio }),
  focusPane: (pane: string) => call<SessionDto>("focus_pane", { pane }),
  openPalette: () => call<SessionDto>("open_palette"),
  closePalette: () => call<SessionDto>("close_palette"),
};
