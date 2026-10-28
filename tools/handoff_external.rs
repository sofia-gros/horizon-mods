use horizon_mods::{
    GameMode, ItemReader, Mods, Output, Package, Runs, ServerHeldStack, ServerPlayer, Snapshot,
    Status, manifest::componentize,
};
use std::{path::PathBuf, sync::Arc};
#[derive(Debug)]
struct Items;
impl ItemReader for Items {
    fn main_hand(&self, _: &str, path: &[String]) -> Result<Option<ServerHeldStack>, String> {
        let text = if path == ["horizon:tool_control", "owner"] {
            "tconstruct_tools"
        } else if path == ["tconstruct", "state_json"] {
            r#"{"schema":1,"tool":"tconstruct:pickaxe","materials":["tconstruct:flint","tconstruct:wood","tconstruct:wood"],"damage":0,"broken":false}"#
        } else {
            "original"
        };
        Ok(Some(ServerHeldStack {
            item: "minecraft:wooden_pickaxe".into(),
            count: 1,
            damage: 0,
            custom_text: Some(text.into()),
        }))
    }
}
#[test]
fn actual_tconstruct_with_welcome_and_trapping_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = PathBuf::from(std::env::var_os("TCONSTRUCT_HMOD").expect("set TCONSTRUCT_HMOD"));
    let welcome = root.join("target/mods-test/mods/welcome.hmod");
    let dir = std::env::temp_dir().join(format!("horizon-multi-mod-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let core = std::fs::read(root.join(
        "target/mods-test/wasm32-unknown-unknown/release/horizon_mods_test_mining_guest.wasm",
    ))
    .unwrap();
    let manifest = "id = \"miner_fixture\"\nname = \"Miner Fixture\"\nversion = \"0.1.0\"\napi = \"0.3\"\nside = \"server\"\ncapabilities = [\"players\", \"storage\", \"items-read\", \"items-edit\", \"items-tools\"]\nexperimental = [\"items-read\", \"items-edit\", \"items-tools\"]\n";
    let trap = dir.join("trap.hmod");
    std::fs::write(
        &trap,
        Package::write(manifest, &componentize(&core).unwrap()).unwrap(),
    )
    .unwrap();
    let welcome_package = Package::read(&welcome).unwrap();
    let duplicate = dir.join("duplicate.hmod");
    let duplicate_manifest = "id = \"tconstruct_tools\"\nname = \"Duplicate Welcome\"\nversion = \"0.1.0\"\napi = \"0.3\"\nside = \"server\"\ncapabilities = [\"players\", \"storage\", \"items-read\", \"items-edit\", \"items-tools\"]\nexperimental = [\"items-read\", \"items-edit\", \"items-tools\"]\n";
    std::fs::write(
        &duplicate,
        Package::write(duplicate_manifest, welcome_package.wasm.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    for paths in [
        [package.clone(), duplicate.clone()],
        [duplicate.clone(), package.clone()],
    ] {
        let collision = Mods::load_for(Runs::Server, &paths, |_| true, |_| false);
        assert_eq!(collision.entries()[0].status, Status::Running);
        assert!(
            matches!(&collision.entries()[1].status, Status::Failed(reason) if reason.contains("another running mod already uses id tconstruct_tools"))
        );
    }
    let mut mods = Mods::load_for(Runs::Server, &[package, welcome, trap], |_| true, |_| false);
    assert!(
        mods.entries().iter().all(|e| e.status == Status::Running),
        "{:?}",
        mods.entries().iter().map(|e| &e.status).collect::<Vec<_>>()
    );
    let p = ServerPlayer {
        id: "abc".into(),
        name: "Alex".into(),
        x: 0.,
        y: 64.,
        z: 0.,
        dimension: "minecraft:overworld".into(),
        game_mode: GameMode::Survival,
        health: 20.,
    };
    mods.set_snapshot(Snapshot {
        players: vec![p.clone()],
        items: Some(Arc::new(Items)),
        ..Snapshot::default()
    });
    assert!(mods.server_commands().any(|c| c.name == "tconstruct_tool"));
    assert!(mods.server_command(&p, "online"));
    assert!(
        mods.drain()
            .iter()
            .any(|o| matches!(o, Output::Reply { error: false, .. }))
    );
    let mine = mods.server_mined(
        "tconstruct_tools",
        &p.id,
        "minecraft:overworld",
        [0, 64, 0],
        "minecraft:stone",
        1.,
    );
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].owner, "tconstruct_tools");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&mine[0].text).unwrap()["damage"],
        1
    );
    let hit = mods.server_melee_hit(
        "tconstruct_tools",
        &p.id,
        "minecraft:overworld",
        1,
        "minecraft:zombie",
        3.,
    );
    assert_eq!(hit.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&hit[0].text).unwrap()["damage"],
        2
    );
    assert!(
        mods.server_mined(
            "miner_fixture",
            &p.id,
            "minecraft:overworld",
            [0, 64, 0],
            "example:trap",
            1.
        )
        .is_empty()
    );
    assert!(matches!(mods.entries()[2].status, Status::Failed(_)));
    assert_eq!(mods.entries()[0].status, Status::Running);
    assert_eq!(mods.entries()[1].status, Status::Running);
    assert_eq!(
        mods.server_mined(
            "tconstruct_tools",
            &p.id,
            "minecraft:overworld",
            [0, 64, 0],
            "minecraft:stone",
            1.
        )
        .len(),
        1
    );
    assert!(mods.server_command(&p, "online"));
    assert!(
        mods.drain().iter().any(
            |o| matches!(o, Output::Reply { error: false, text, .. } if text.contains("Alex"))
        )
    );
    std::fs::remove_dir_all(dir).unwrap();
}
