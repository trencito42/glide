export interface TabDto {
  id: string;
  url: string;
  title: string;
  pinned: boolean;
  loading: boolean;
  realized: boolean;
}

export interface SpaceDto {
  id: string;
  name: string;
  activeTabId: string;
  focusedPane: string;
  splitTabId: string | null;
  splitRatio: number;
  tabs: TabDto[];
}

export interface SessionDto {
  theme: "graphite" | "porcelain" | string;
  layout: "horizontal" | "sidebar" | string;
  activeSpaceId: string;
  paletteOpen: boolean;
  spaces: SpaceDto[];
}

export function activeSpace(session: SessionDto): SpaceDto | undefined {
  return session.spaces.find((space) => space.id === session.activeSpaceId);
}

export function focusedTab(space: SpaceDto): TabDto | undefined {
  const id = space.focusedPane === "secondary" && space.splitTabId ? space.splitTabId : space.activeTabId;
  return space.tabs.find((tab) => tab.id === id);
}
