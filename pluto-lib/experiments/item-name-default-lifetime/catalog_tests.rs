// Source-only tests for the actual official items/world crates; not yet compiled.
// Intended as a separate future fixture module, never added to frozen codec32.
use horizon_items::{item_data::encode_components, items::{ItemStack, Items}};
use horizon_world::{item_defaults, loot::PackOverlay, nbt::Tag};
use std::{fs, path::PathBuf, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

const ITEM: &str = "example:default_name_probe";
const NAME: &str = "minecraft:item_name";
const CUSTOM: &str = "example:version_probe";
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("pluto-name-default-{}-{nonce}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&path).unwrap();
        let result = Self(path);
        fs::create_dir_all(result.0.join("data/minecraft/loot_table")).unwrap();
        result
    }
    fn write(&self, version: &str) {
        let component = serde_json::json!({"translate": format!("item.example.{version}"), "color":"#123456", "italic":false});
        let json = serde_json::json!({(ITEM): {(NAME):component, (CUSTOM):version, "minecraft:max_stack_size":64}});
        fs::write(self.0.join("items.json"), serde_json::to_vec(&json).unwrap()).unwrap();
    }
    fn catalog(&self) -> Items { Items::catalog(&self.0, &PackOverlay::NONE).unwrap() }
}
impl Drop for Root {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn version(items: &Items) -> &Tag { &items.get(ITEM).unwrap().custom_components[CUSTOM] }

#[test]
fn catalog_retains_key_but_not_standard_name_value() {
    let root = Root::new(); root.write("one");
    let raw = item_defaults::shared(&root.0).unwrap();
    assert_eq!(raw[ITEM][NAME]["translate"], "item.example.one");
    let items = root.catalog(); let info = items.get(ITEM).unwrap();
    assert!(info.components.iter().any(|key| key == NAME));
    assert!(!info.custom_components.contains_key(NAME));
    assert_eq!(version(&items), &Tag::String("one".into()));
    let stack = ItemStack::new(ITEM, 1);
    assert!(stack.data().is_none()); assert!(stack.item_name().is_none());
}

#[test]
fn holding_native_snapshot_keeps_old_defaults_until_release() {
    let root = Root::new(); root.write("one");
    let first = item_defaults::shared(&root.0).unwrap();
    root.write("two");
    let second = item_defaults::shared(&root.0).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    let old_items = root.catalog();
    assert_eq!(version(&old_items), &Tag::String("one".into()));
    // An explicit fresh read is not the snapshot used by Items::catalog.
    assert_eq!(item_defaults::read(&root.0).unwrap()[ITEM][NAME]["translate"], "item.example.two");
    drop((first, second));
    let new_items = root.catalog();
    assert_eq!(version(&new_items), &Tag::String("two".into()));
    assert_eq!(version(&old_items), &Tag::String("one".into()));
}

#[test]
fn catalog_does_not_retain_native_defaults_snapshot() {
    let root = Root::new(); root.write("one");
    let raw = item_defaults::shared(&root.0).unwrap(); let weak = Arc::downgrade(&raw);
    let items = root.catalog(); drop(raw);
    assert!(weak.upgrade().is_none());
    root.write("two");
    let fresh = item_defaults::shared(&root.0).unwrap();
    assert_eq!(fresh[ITEM][NAME]["translate"], "item.example.two");
    assert_eq!(version(&items), &Tag::String("one".into()));
}

#[test]
fn display_reads_do_not_materialize_prototype_or_change_patch() {
    let root = Root::new(); root.write("one"); let _items = root.catalog();
    let plain = ItemStack::new(ITEM, 1); let encoded_plain = encode_components(&plain);
    assert!(plain.item_name().is_none()); assert_eq!(encode_components(&plain), encoded_plain);
    let mut explicit = plain.clone();
    explicit.data_mut().set_other(NAME, Some(Tag::String("Explicit name".into())));
    let saved = encode_components(&explicit); let before = explicit.clone();
    assert_eq!(explicit.item_name().as_deref(), Some("Explicit name"));
    assert_eq!(explicit, before); assert_eq!(encode_components(&explicit), saved);
    let mut removed = plain.clone();
    removed.data_mut().set_other("!minecraft:item_name", Some(Tag::Compound(Default::default())));
    removed.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Custom wins".into())));
    let saved = encode_components(&removed); let before = removed.clone();
    assert!(removed.item_name().is_none());
    assert_eq!(removed.custom_name().as_deref(), Some("Custom wins"));
    assert_eq!(removed, before); assert_eq!(encode_components(&removed), saved);
}
