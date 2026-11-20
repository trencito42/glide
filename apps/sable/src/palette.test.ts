import { expect, test } from "vitest";
import { commandItems } from "./palette";
import type { SessionDto } from "./types";

const session: SessionDto = {
  theme: "graphite",
  layout: "horizontal",
  activeSpaceId: "s1",
  paletteOpen: true,
  spaces: [
    {
      id: "s1",
      name: "Personal",
      activeTabId: "t1",
      focusedPane: "primary",
      splitTabId: null,
      splitRatio: 0.5,
      tabs: [
        { id: "t1", url: "sable:newtab", title: "New tab", pinned: false, loading: false, realized: true },
        { id: "t2", url: "https://example.com", title: "Example", pinned: true, loading: false, realized: true },
      ],
    },
    {
      id: "s2",
      name: "Work",
      activeTabId: "t3",
      focusedPane: "primary",
      splitTabId: null,
      splitRatio: 0.5,
      tabs: [{ id: "t3", url: "sable:newtab", title: "New tab", pinned: false, loading: false, realized: false }],
    },
  ],
};

test("the command list includes the active space's tabs and not the other space", () => {
  const values = commandItems(session).map((item) => item.value);
  expect(values).toContain("tab:t1");
  expect(values).toContain("tab:t2");
  expect(values).not.toContain("tab:t3");
  expect(values).toContain("space:s2");
});
