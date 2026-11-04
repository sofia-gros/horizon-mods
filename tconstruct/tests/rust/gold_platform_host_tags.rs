use horizon_items::items::Items;
use horizon_world::loot::PackOverlay;
use std::{fs, path::PathBuf};
#[test]
fn actual_items_catalog_loads_gold_tags_and_honors_overlay_replace() {
 let root: PathBuf = std::env::temp_dir().join(format!("gold-platform-tags-{}", std::process::id()));
 let _ = fs::remove_dir_all(&root);
 for (path, bytes) in [
  ("horizon-mod-content.json", br#"["tconstruct","forge"]"#.as_slice()),
  ("items.json", br#"{}"#.as_slice()),
  ("data/forge/tags/item/ingots/gold.json", include_bytes!("../../content/data/forge/tags/item/ingots/gold.json").as_slice()),
  ("data/forge/tags/item/nuggets/gold.json", include_bytes!("../../content/data/forge/tags/item/nuggets/gold.json").as_slice()),
 ] { let path = root.join(path); fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, bytes).unwrap(); }
 let initial = Items::catalog(&root, &PackOverlay::NONE).unwrap();
 assert_eq!(initial.item_tag_order["forge:ingots/gold"], ["minecraft:gold_ingot"]);
 assert_eq!(initial.item_tag_order["forge:nuggets/gold"], ["minecraft:gold_nugget"]);
 let addition = vec![("tags/forge:item/ingots/gold".to_owned(), vec![br#"{"values":["example:gold"]}"#.to_vec()])];
 let merged = Items::catalog(&root, &PackOverlay { files: &addition }).unwrap();
 assert_eq!(merged.item_tag_order["forge:ingots/gold"], ["minecraft:gold_ingot", "example:gold"]);
 let files = vec![("tags/forge:item/ingots/gold".to_owned(), vec![br#"{"replace":true,"values":["example:gold"]}"#.to_vec()])];
 let overlay = PackOverlay { files: &files };
 let reloaded = Items::catalog(&root, &overlay).unwrap();
 assert_eq!(reloaded.item_tag_order["forge:ingots/gold"], ["example:gold"]);
 assert_eq!(reloaded.item_tag_order["forge:nuggets/gold"], ["minecraft:gold_nugget"]);
 fs::remove_dir_all(root).unwrap();
}
