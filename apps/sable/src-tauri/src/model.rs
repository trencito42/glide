use serde::{Deserialize, Serialize};

pub const NEW_TAB_URL: &str = "sable:newtab";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub theme: String,
    pub layout: String,
    pub active_space_id: String,
    pub next_id: u64,
    pub spaces: Vec<Space>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub active_tab_id: String,
    pub focused_pane: String,
    pub split_tab_id: Option<String>,
    pub split_ratio: f64,
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(skip)]
    pub loading: bool,
    #[serde(skip)]
    pub realized: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub theme: String,
    pub layout: String,
    pub active_space_id: String,
    pub palette_open: bool,
    pub spaces: Vec<SpaceDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceDto {
    pub id: String,
    pub name: String,
    pub active_tab_id: String,
    pub focused_pane: String,
    pub split_tab_id: Option<String>,
    pub split_ratio: f64,
    pub tabs: Vec<TabDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabDto {
    pub id: String,
    pub url: String,
    pub title: String,
    pub pinned: bool,
    pub loading: bool,
    pub realized: bool,
}

impl Session {
    pub fn fresh() -> Self {
        let mut session = Self {
            theme: "graphite".into(),
            layout: "horizontal".into(),
            active_space_id: String::new(),
            next_id: 1,
            spaces: Vec::new(),
        };
        let personal = session.make_space("Personal");
        let work = session.make_space("Work");
        session.active_space_id = personal;
        let _ = work;
        session
    }

    pub fn alloc(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub fn make_space(&mut self, name: &str) -> String {
        let id = self.alloc("s");
        let tab = self.make_tab();
        let tab_id = tab.id.clone();
        self.spaces.push(Space {
            id: id.clone(),
            name: name.into(),
            active_tab_id: tab_id,
            focused_pane: "primary".into(),
            split_tab_id: None,
            split_ratio: 0.5,
            tabs: vec![tab],
        });
        id
    }

    pub fn make_tab(&mut self) -> Tab {
        let id = self.alloc("t");
        Tab {
            id,
            url: NEW_TAB_URL.into(),
            title: "New tab".into(),
            pinned: false,
            loading: false,
            realized: false,
        }
    }

    pub fn active_space_mut(&mut self) -> Result<&mut Space, String> {
        let id = self.active_space_id.clone();
        self.spaces
            .iter_mut()
            .find(|space| space.id == id)
            .ok_or_else(|| "active space is missing".into())
    }

    pub fn active_space(&self) -> Result<&Space, String> {
        self.spaces
            .iter()
            .find(|space| space.id == self.active_space_id)
            .ok_or_else(|| "active space is missing".into())
    }

    pub fn dto(&self, palette_open: bool) -> SessionDto {
        SessionDto {
            theme: self.theme.clone(),
            layout: self.layout.clone(),
            active_space_id: self.active_space_id.clone(),
            palette_open,
            spaces: self.spaces.iter().map(Space::dto).collect(),
        }
    }
}

impl Space {
    pub fn dto(&self) -> SpaceDto {
        SpaceDto {
            id: self.id.clone(),
            name: self.name.clone(),
            active_tab_id: self.active_tab_id.clone(),
            focused_pane: self.focused_pane.clone(),
            split_tab_id: self.split_tab_id.clone(),
            split_ratio: self.split_ratio,
            tabs: self.tabs.iter().map(Tab::dto).collect(),
        }
    }

    pub fn focused_tab_id(&self) -> &str {
        if self.focused_pane == "secondary" {
            if let Some(id) = self.split_tab_id.as_deref() {
                return id;
            }
        }
        &self.active_tab_id
    }

    pub fn tab_mut(&mut self, id: &str) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }

    pub fn tab(&self, id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }
}

impl Tab {
    pub fn dto(&self) -> TabDto {
        TabDto {
            id: self.id.clone(),
            url: self.url.clone(),
            title: self.title.clone(),
            pinned: self.pinned,
            loading: self.loading,
            realized: self.realized,
        }
    }
}

pub fn partition_pins(tabs: &mut [Tab]) {
    tabs.sort_by_key(|tab| !tab.pinned);
}

pub fn move_tab(tabs: &mut Vec<Tab>, id: &str, to: usize) -> Result<(), String> {
    let from = tabs
        .iter()
        .position(|tab| tab.id == id)
        .ok_or_else(|| "tab not found".to_string())?;
    let pinned = tabs[from].pinned;
    let pinned_count = tabs.iter().filter(|tab| tab.pinned).count();
    let start = if pinned { 0 } else { pinned_count };
    let end = if pinned { pinned_count } else { tabs.len() };
    if end <= start {
        return Ok(());
    }
    let dest = to.clamp(start, end - 1);
    if from == dest {
        return Ok(());
    }
    let tab = tabs.remove(from);
    let dest = if dest > from { dest - 1 } else { dest };
    tabs.insert(dest.min(tabs.len()), tab);
    Ok(())
}

pub fn cycle_tab(tabs: &[Tab], active: &str, backward: bool) -> Option<String> {
    if tabs.is_empty() {
        return None;
    }
    let index = tabs.iter().position(|tab| tab.id == active).unwrap_or(0);
    let count = tabs.len();
    let next = if backward {
        (index + count - 1) % count
    } else {
        (index + 1) % count
    };
    Some(tabs[next].id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str, pinned: bool) -> Tab {
        Tab {
            id: id.into(),
            url: NEW_TAB_URL.into(),
            title: id.into(),
            pinned,
            loading: false,
            realized: false,
        }
    }

    #[test]
    fn pins_stay_in_their_group_when_moved() {
        let mut tabs = vec![tab("a", true), tab("b", true), tab("c", false), tab("d", false)];
        move_tab(&mut tabs, "d", 0).unwrap();
        let ids: Vec<_> = tabs.iter().map(|tab| tab.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "d", "c"]);
    }

    #[test]
    fn pinning_repartitions_without_scrambling_peers() {
        let mut tabs = vec![tab("a", true), tab("b", false), tab("c", false)];
        tabs[2].pinned = true;
        partition_pins(&mut tabs);
        let ids: Vec<_> = tabs.iter().map(|tab| tab.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c", "b"]);
    }

    #[test]
    fn cycle_wraps() {
        let tabs = vec![tab("a", false), tab("b", false), tab("c", false)];
        assert_eq!(cycle_tab(&tabs, "c", false).as_deref(), Some("a"));
        assert_eq!(cycle_tab(&tabs, "a", true).as_deref(), Some("c"));
    }
}
