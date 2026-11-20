import { AnimatedIndicator, IconButton, ScrollArea } from "@b4/react";
import { useId, useState } from "react";
import { browser } from "./api";
import { Icon } from "./icons";
import { hostLabel, tabLetter } from "./letter";
import type { TabDto } from "./types";

export function TabList({
  tabs,
  activeId,
  orientation,
}: {
  tabs: TabDto[];
  activeId: string;
  orientation: "horizontal" | "vertical";
}) {
  const indicatorId = useId();
  const [dragging, setDragging] = useState<string | null>(null);

  return (
    <ScrollArea className="tab-scroll" maxHeight={orientation === "vertical" ? "100%" : undefined}>
      <div className={orientation === "vertical" ? "tab-list vertical" : "tab-list"} role="tablist" aria-label="Tabs">
        {tabs.map((tab, index) => {
          const selected = tab.id === activeId;
          const label = tab.url.startsWith("sable:") ? "New tab" : tab.title || hostLabel(tab.url) || "New tab";
          return (
            <div
              key={tab.id}
              role="tab"
              aria-selected={selected}
              tabIndex={selected ? 0 : -1}
              className={tab.pinned ? "tab pinned" : "tab"}
              data-dragging={dragging === tab.id ? "true" : undefined}
              data-loading={tab.loading ? "true" : undefined}
              draggable
              onClick={() => void browser.selectTab(tab.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void browser.selectTab(tab.id);
                }
              }}
              onDragStart={(event) => {
                event.dataTransfer.setData("text/plain", tab.id);
                event.dataTransfer.effectAllowed = "move";
                setDragging(tab.id);
              }}
              onDragEnd={() => setDragging(null)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();
                const id = event.dataTransfer.getData("text/plain");
                if (id) void browser.moveTab(id, index);
                setDragging(null);
              }}
            >
              {selected ? <AnimatedIndicator layoutId={indicatorId} className="tab-pill" /> : null}
              <span className="tab-body">
                <span className="tab-mark" aria-hidden="true">
                  {tabLetter(label, tab.url)}
                </span>
                {tab.pinned ? null : <span className="tab-title">{label}</span>}
              </span>
              <span className="tab-actions">
                <button
                  type="button"
                  className="tab-icon"
                  aria-label={tab.pinned ? `Unpin ${label}` : `Pin ${label}`}
                  aria-pressed={tab.pinned}
                  onClick={(event) => {
                    event.stopPropagation();
                    void browser.pinTab(tab.id, !tab.pinned);
                  }}
                >
                  <Icon name="pin" />
                </button>
                <button
                  type="button"
                  className="tab-icon"
                  aria-label={`Close ${label}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    void browser.closeTab(tab.id);
                  }}
                  onAuxClick={(event) => {
                    if (event.button === 1) {
                      event.preventDefault();
                      void browser.closeTab(tab.id);
                    }
                  }}
                >
                  <Icon name="close" />
                </button>
              </span>
            </div>
          );
        })}
        <IconButton
          variant="ghost"
          size="sm"
          aria-label="New tab"
          icon={<Icon name="plus" />}
          onClick={() => void browser.newTab()}
        />
      </div>
    </ScrollArea>
  );
}
