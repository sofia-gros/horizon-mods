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
}
