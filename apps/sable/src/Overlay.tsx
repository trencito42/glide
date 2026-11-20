import { CommandPalette } from "@b4/react";
import { browser } from "./api";
import { commandItems } from "./palette";
import type { SessionDto } from "./types";
import { activeSpace } from "./types";

export function Overlay({ session }: { session: SessionDto }) {
  const space = activeSpace(session);
  return (
    <CommandPalette
      aria-label="Tabs and commands"
      open
      items={commandItems(session)}
      placeholder="Search open tabs"
      onOpenChange={(open) => {
        if (!open) void browser.closePalette();
      }}
      onSelect={(value) => {
        const tab = space?.tabs.find((entry) => entry.id === space.activeTabId);
        void runCommand(value, space?.activeTabId, tab?.pinned ?? false);
      }}
    />
  );
}

async function runCommand(value: string, activeTabId: string | undefined, pinned: boolean) {
  if (value === "action:new-tab") await browser.newTab();
  else if (value === "action:close-tab" && activeTabId) await browser.closeTab(activeTabId);
  else if (value === "action:split") await browser.toggleSplit();
  else if (value === "action:pin" && activeTabId) await browser.pinTab(activeTabId, !pinned);
  else if (value.startsWith("space:")) await browser.selectSpace(value.slice(6));
  else if (value.startsWith("layout:")) await browser.setLayout(value.slice(7));
  else if (value.startsWith("theme:")) await browser.setTheme(value.slice(6));
  else if (value.startsWith("tab:")) await browser.selectTab(value.slice(4));
  await browser.closePalette();
}
