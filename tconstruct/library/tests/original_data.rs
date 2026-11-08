//! Loads the original's generated data (`src/generated/resources/data/tconstruct/tinkering`) with the library, when the original's source is
//! checked out under `.local/tconstruct-source` (as the owner's machine has it); otherwise the test does nothing.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tconstruct_library::condition::EmptyContext;
use tconstruct_library::materials::manager::MaterialManager;
use tconstruct_library::materials::traits::MaterialTraitsManager;
use tconstruct_library::materials::MaterialId;
use tconstruct_library::resource::ResourceId;

fn data_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering");
    root.is_dir().then_some(root)
}

/// Every `.json` under `dir`, as (`tconstruct:<relative path without extension>`, parsed).
fn read_all(dir: &Path) -> Vec<(ResourceId, Value)> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(folder) = stack.pop() {
        let Ok(entries) = fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let relative = path.strip_prefix(dir).unwrap().with_extension("");
                let name = relative.to_string_lossy().replace('\\', "/");
                let json: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                found.push((ResourceId::try_new("tconstruct", &name).unwrap(), json));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn the_originals_materials_load_without_problems() {
    let Some(root) = data_root() else { return };
    let definitions = read_all(&root.join("materials/definition"));
    let tags = read_all(&root.join("tags/materials"));
    assert!(definitions.len() > 100, "{} definition files", definitions.len());
    let (manager, problems) = MaterialManager::load(&definitions, &tags, &EmptyContext);
    assert!(problems.is_empty(), "{problems:?}");
    let flint = manager.material(&MaterialId::try_parse("tconstruct:flint").unwrap()).unwrap();
    assert_eq!((flint.tier, flint.craftable), (1, true));
    // nothing is installed beside the game and no tag has members, so the integration materials stay out
    assert!(manager.material(&MaterialId::try_parse("tconstruct:aluminum").unwrap()).is_none());
    assert!(manager.visible_materials().windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn the_originals_traits_load_without_problems() {
    let Some(root) = data_root() else { return };
    let files = read_all(&root.join("materials/traits"));
    assert!(files.len() > 90, "{} trait files", files.len());
    let mut manager = MaterialTraitsManager::default();
    let problems = manager.load(&files);
    assert!(problems.is_empty(), "{problems:?}");
    let wood = MaterialId::try_parse("tconstruct:wood").unwrap();
    assert_eq!(manager.default_traits(&wood)[0].modifier.to_string(), "tconstruct:cultivated");
}
