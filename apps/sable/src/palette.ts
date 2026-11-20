import type { CommandPaletteItem } from "@b4/react";
import { hostLabel } from "./letter";
import { activeSpace, type SessionDto } from "./types";

export function commandItems(session: SessionDto): CommandPaletteItem[] {
  const space = activeSpace(session);
  const items: CommandPaletteItem[] = [
    { value: "action:new-tab", label: "New tab", shortcut: ["Ctrl", "T"] },
    { value: "action:close-tab", label: "Close tab", shortcut: ["Ctrl", "W"] },
    {
      value: "action:split",
      label: space?.splitTabId ? "Close split view" : "Split view",
      disabled: !space || space.tabs.length < 2,
    },
    { value: "action:pin", label: "Pin or unpin this tab" },
  ];
  for (const entry of session.spaces) {
    items.push({
      value: `space:${entry.id}`,
      label: entry.name,
      description: entry.id === session.activeSpaceId ? "Current space" : "Switch space",
    });
  }
  items.push(
    { value: "layout:horizontal", label: "Tabs across the top" },
    { value: "layout:sidebar", label: "Tabs down the side" },
    { value: "theme:graphite", label: "Graphite" },
    { value: "theme:porcelain", label: "Porcelain" },
  );
  for (const tab of space?.tabs ?? []) {
    items.push({
      value: `tab:${tab.id}`,
      label: tab.title || "New tab",
      description: tab.url.startsWith("sable:") ? "New tab" : hostLabel(tab.url) || tab.url,
    });
  }
  return items;
}
