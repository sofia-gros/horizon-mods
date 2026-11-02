//! Generate Horizon content declarations and prepare Creative tabs.
//! See the guide for the distinction between content files and Creative data.
use serde_json::{json, Value};
use std::{collections::BTreeSet, error::Error, fmt, fs, path::Path};

#[derive(Debug)]
pub struct RegistrationError(pub String);
impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for RegistrationError {}
impl From<std::io::Error> for RegistrationError {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}

fn part(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
}
fn id(s: &str) -> bool {
    s.split_once(':')
        .is_some_and(|(ns, path)| part(ns) && path.split('/').all(part))
}
fn vanilla(s: &str) -> bool {
    id(s) && s.starts_with("minecraft:")
}

/// A single content mod's generated declarations. Duplicate IDs are rejected.
pub struct Registry {
    namespace: String,
    entries: Vec<(String, String, Value)>,
    ids: BTreeSet<String>,
}
impl Registry {
    pub fn new(namespace: &str) -> Result<Self, RegistrationError> {
        if !part(namespace) || namespace == "minecraft" {
            return Err(RegistrationError("invalid mod namespace".into()));
        }
        Ok(Self {
            namespace: namespace.into(),
            entries: Vec::new(),
            ids: BTreeSet::new(),
        })
    }
    /// Add an item based on a vanilla item template.
    pub fn item(&mut self, name: &str, template: &str) -> Result<(), RegistrationError> {
        self.add("item", name, template, json!({"template": template}))
    }
    /// Add a block and its default block item based on a vanilla block template.
    pub fn block(&mut self, name: &str, template: &str) -> Result<(), RegistrationError> {
        self.add("block", name, template, json!({"template": template}))
    }
    fn add(
        &mut self,
        kind: &str,
        name: &str,
        template: &str,
        value: Value,
    ) -> Result<(), RegistrationError> {
        if !name.split('/').all(part) || !vanilla(template) {
            return Err(RegistrationError("invalid name or vanilla template".into()));
        }
        // A block creates an item with the same ID by default.
        if !self.ids.insert(name.into()) {
            return Err(RegistrationError(format!(
                "duplicate item or block ID: {name}"
            )));
        }
        self.entries.push((kind.into(), name.into(), value));
        Ok(())
    }
    /// Write declarations into a new directory; never overwrites an existing tree.
    pub fn write_to(&self, root: &Path) -> Result<(), RegistrationError> {
        fs::create_dir(root)?;
        for (kind, name, value) in &self.entries {
            let file = root
                .join("data")
                .join(&self.namespace)
                .join("horizon")
                .join(kind)
                .join(format!("{name}.json"));
            fs::create_dir_all(file.parent().unwrap())?;
            fs::write(file, serde_json::to_vec_pretty(value).unwrap())?;
        }
        Ok(())
    }
}

/// A tab for the existing seven-column Creative screen. Stacks use Horizon's SNBT format.
pub struct CreativeTab<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub icon: &'a str,
    pub row: &'a str,
    pub column: u8,
    pub items: Vec<&'a str>,
}
/// Return a modified copy of a prepared Creative inventory; the input is unchanged.
pub fn add_creative_tabs(
    inventory: &Value,
    groups: &[CreativeTab<'_>],
) -> Result<Value, RegistrationError> {
    let mut out = inventory.clone();
    let tabs = out
        .get_mut("tabs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| RegistrationError("missing tabs array".into()))?;
    for tab in groups {
        if !id(tab.id)
            || tab.title.is_empty()
            || tab.icon.is_empty()
            || !matches!(tab.row, "TOP" | "BOTTOM")
            || tab.column > 6
        {
            return Err(RegistrationError("invalid Creative tab".into()));
        }
        if tabs.iter().any(|t| {
            t.get("id").and_then(Value::as_str) == Some(tab.id)
                || (t.get("type").and_then(Value::as_str) != Some("HOTBAR")
                    && t.get("visible").and_then(Value::as_bool) != Some(false)
                    && t.get("row").and_then(Value::as_str) == Some(tab.row)
                    && t.get("column").and_then(Value::as_u64) == Some(tab.column.into()))
        }) {
            return Err(RegistrationError(
                "Creative tab ID or position already used".into(),
            ));
        }
        tabs.push(json!({"id":tab.id,"title":tab.title,"icon":tab.icon,"row":tab.row,"column":tab.column,"items":tab.items,"type":"CATEGORY","visible":true}));
    }
    Ok(out)
}

/// An additional tab whose screen position is assigned by PlutoLib's paging adapter.
pub struct PagedCreativeTab<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub icon: &'a str,
    pub items: Vec<&'a str>,
}

/// Append tabs for the PlutoLib Creative paging Mixins bundle.
/// Without that bundle, overflow tabs overlap and cannot all be reached.
pub fn add_paged_creative_tabs(
    inventory: &Value,
    groups: &[PagedCreativeTab<'_>],
) -> Result<Value, RegistrationError> {
    let mut out = inventory.clone();
    let tabs = out
        .get_mut("tabs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| RegistrationError("missing tabs array".into()))?;
    let mut ids: BTreeSet<String> = tabs
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let mut occupied: BTreeSet<(String, u64)> = tabs
        .iter()
        .filter(|t| {
            t.get("type").and_then(Value::as_str) != Some("HOTBAR")
                && t.get("visible").and_then(Value::as_bool) != Some(false)
        })
        .filter_map(|t| {
            Some((
                t.get("row")?.as_str()?.to_owned(),
                t.get("column")?.as_u64()?,
            ))
        })
        .collect();
    for tab in groups {
        if !id(tab.id)
            || tab.title.is_empty()
            || tab.icon.is_empty()
            || !ids.insert(tab.id.to_owned())
        {
            return Err(RegistrationError(
                "invalid or duplicate Creative tab".into(),
            ));
        }
        let (row, column) = ["TOP", "BOTTOM"]
            .into_iter()
            .flat_map(|row| (0..7).map(move |column| (row, column)))
            .find(|(row, column)| !occupied.contains(&(row.to_string(), *column)))
            .unwrap_or(("TOP", 0));
        occupied.insert((row.to_owned(), column));
        tabs.push(json!({"id":tab.id,"title":tab.title,"icon":tab.icon,"row":row,"column":column,"items":tab.items,"type":"CATEGORY","visible":true}));
    }
    Ok(out)
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../mixins/pluto_creative/paging.rs"]
mod paging;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_registration_and_collisions() {
        let mut r = Registry::new("demo").unwrap();
        r.block("stone", "minecraft:stone").unwrap();
        assert!(r.item("stone", "minecraft:stick").is_err());
        assert!(r.item("../escape", "minecraft:stick").is_err());
        assert!(r.item("stick", "demo:stick").is_err());
    }
    #[test]
    fn creative_preserves_input_order_and_rejects_collisions() {
        let base =
            json!({"tabs": [{"id":"minecraft:hotbar","type":"HOTBAR","row":"TOP","column":5}]});
        let tab = CreativeTab {
            id: "demo:things",
            title: "Things",
            icon: "{count:1,id:\"demo:gem\"}",
            row: "TOP",
            column: 5,
            items: vec!["first", "second"],
        };
        let output = add_creative_tabs(&base, &[tab]).unwrap();
        assert_eq!(base["tabs"].as_array().unwrap().len(), 1);
        assert_eq!(output["tabs"][1]["items"], json!(["first", "second"]));
        let collision = CreativeTab {
            id: "demo:other",
            title: "Other",
            icon: "x",
            row: "TOP",
            column: 5,
            items: vec![],
        };
        assert!(add_creative_tabs(&output, &[collision]).is_err());
    }
    #[test]
    fn paging_visits_all_tabs_and_restores_selection() {
        let mut pages = super::paging::Pages::default();
        let tabs: Vec<_> = (0..31).collect();
        assert!(pages.initialize(&tabs));
        assert_eq!(pages.count(), 3);
        assert_eq!(pages.visible(), (0..14).collect::<Vec<_>>());
        assert_eq!(pages.turn(1, 7).unwrap(), ((14..28).collect(), 0));
        assert_eq!(pages.turn(1, 5).unwrap(), ((28..31).collect(), 0));
        assert!(pages.turn(1, 0).is_none());
        assert_eq!(pages.turn(-1, 2).unwrap(), ((14..28).collect(), 5));
        assert_eq!(pages.turn(-1, 5).unwrap(), ((0..14).collect(), 7));
        assert!(pages.turn(-1, 7).is_none());
    }
    #[test]
    fn paged_tabs_use_free_positions_before_overflow() {
        let base = json!({"tabs": [
            {"id":"minecraft:building", "row":"TOP", "column":0},
            {"id":"minecraft:hotbar", "type":"HOTBAR", "row":"TOP", "column":1}
        ]});
        let tab = PagedCreativeTab {
            id: "demo:new",
            title: "New",
            icon: "x",
            items: vec![],
        };
        let output = add_paged_creative_tabs(&base, &[tab]).unwrap();
        assert_eq!(output["tabs"][2]["column"], 1);
        assert_eq!(output["tabs"][2]["row"], "TOP");
    }
    #[test]
    fn paged_tabs_reject_duplicate_ids_without_mutating_input() {
        let base = json!({"tabs": [{"id":"demo:first"}]});
        let tab = PagedCreativeTab {
            id: "demo:first",
            title: "First",
            icon: "x",
            items: vec![],
        };
        assert!(add_paged_creative_tabs(&base, &[tab]).is_err());
        assert_eq!(base["tabs"].as_array().unwrap().len(), 1);
    }
}
