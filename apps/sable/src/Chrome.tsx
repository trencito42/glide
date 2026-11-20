import { IconButton, SegmentedControl, Slider, TextField } from "@b4/react";
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { browser } from "./api";
import { Icon } from "./icons";
import { omniboxValue } from "./letter";
import { TabList } from "./TabList";
import { activeSpace, focusedTab, type SessionDto } from "./types";

export function Chrome({ session }: { session: SessionDto }) {
  const space = activeSpace(session);
  const current = space ? focusedTab(space) : undefined;
  const shown = omniboxValue(current?.url ?? "");
  const [draft, setDraft] = useState(shown);
  const [editing, setEditing] = useState(false);
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!editing) setDraft(shown);
  }, [shown, editing]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("omnibox-focus", () => {
      setEditing(true);
      field.current?.focus();
      field.current?.select();
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, []);

  if (!space) return null;
  const sidebar = session.layout === "sidebar";

  return (
    <div className={sidebar ? "chrome side" : "chrome top"}>
      <div className="clip-probe">Clip probe</div>
      <div className="tab-row">
        <TabList
          tabs={space.tabs}
          activeId={space.activeTabId}
          orientation={sidebar ? "vertical" : "horizontal"}
        />
      </div>
      <div className="toolbar">
        <IconButton variant="ghost" size="sm" aria-label="Back" icon={<Icon name="back" />} onClick={() => void browser.back()} />
        <IconButton variant="ghost" size="sm" aria-label="Forward" icon={<Icon name="forward" />} onClick={() => void browser.forward()} />
        <IconButton variant="ghost" size="sm" aria-label="Reload" icon={<Icon name="reload" />} onClick={() => void browser.reload()} />
        <TextField
          ref={field}
          className="omnibox"
          label="Address"
          hideLabel
          size="sm"
          value={draft}
          placeholder="Search or enter address"
          spellCheck={false}
          autoCapitalize="off"
          onFocus={() => setEditing(true)}
          onBlur={() => setEditing(false)}
          onValueChange={setDraft}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void browser.navigate(draft);
              field.current?.blur();
            }
            if (event.key === "Escape") {
              setDraft(shown);
              field.current?.blur();
            }
          }}
        />
        <IconButton
          variant="ghost"
          size="sm"
          aria-label="Command palette"
          icon={<Icon name="split" />}
          onClick={() => void browser.openPalette()}
        />
        <SegmentedControl
          aria-label="Spaces"
          size="sm"
          value={space.id}
          items={session.spaces.map((entry) => ({ value: entry.id, label: entry.name }))}
          onValueChange={(value) => void browser.selectSpace(value)}
        />
        <SegmentedControl
          aria-label="Tab layout"
          size="sm"
          value={sidebar ? "sidebar" : "horizontal"}
          items={[
            { value: "horizontal", label: "Top" },
            { value: "sidebar", label: "Side" },
          ]}
          onValueChange={(value) => void browser.setLayout(value)}
        />
        <SegmentedControl
          aria-label="Theme"
          size="sm"
          value={session.theme === "porcelain" ? "porcelain" : "graphite"}
          items={[
            { value: "graphite", label: "Graphite" },
            { value: "porcelain", label: "Porcelain" },
          ]}
          onValueChange={(value) => void browser.setTheme(value)}
        />
        <IconButton
          variant={space.splitTabId ? "secondary" : "ghost"}
          size="sm"
          aria-label={space.splitTabId ? "Close split view" : "Split view"}
          aria-pressed={Boolean(space.splitTabId)}
          disabled={space.tabs.length < 2 && !space.splitTabId}
          icon={<Icon name="split" />}
          onClick={() => void browser.toggleSplit()}
        />
        {space.splitTabId ? (
          <div className="split-tools">
            <button type="button" className={space.focusedPane === "primary" ? "pane on" : "pane"} onClick={() => void browser.focusPane("primary")}>
              Left
            </button>
            <Slider
              label="Split width"
              size="sm"
              min={0.25}
              max={0.75}
              step={0.01}
              value={space.splitRatio}
              onValueChange={(value) => {
                if (typeof value === "number") void browser.setSplitRatio(value);
              }}
            />
            <button type="button" className={space.focusedPane === "secondary" ? "pane on" : "pane"} onClick={() => void browser.focusPane("secondary")}>
              Right
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
