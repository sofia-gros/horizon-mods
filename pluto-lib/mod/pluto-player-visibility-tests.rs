// Compile the actual native adapter body against a reduced snapshot, not a reimplementation. This tests branching/math only, not
// native type bindings.
use crate::mobs;
struct PlayerView { uuid: [i32; 4] }
include!("pluto-player-visibility.rs");
#[test]
fn actual_visibility_adapter_scales_only_known_kinds_and_keeps_the_native_value_otherwise() {
    let player = PlayerView { uuid: [-1, 2, i32::MIN, 4] };
    // No sink yet: the native value stands.
    assert_eq!(pluto_player_visibility(0.8, &player, Some("minecraft:skeleton")), 0.8);
    mobs::pluto_player_visibility_access::set_sink(mobs::pluto_player_visibility_access::Sink { factor: |wearer, kind| {
        assert_eq!(wearer, 0xffff_ffff_0000_0002_8000_0000_0000_0004);
        match kind { "minecraft:skeleton" => 0.25, "minecraft:nan" => f64::NAN, "minecraft:negative" => -3.0, _ => 1.0 }
    } }).unwrap();
    assert!(mobs::pluto_player_visibility_access::set_sink(mobs::pluto_player_visibility_access::Sink { factor: |_, _| 1.0 }).is_err());
    assert_eq!(pluto_player_visibility(0.8, &player, Some("minecraft:skeleton")), 0.2);
    assert_eq!(pluto_player_visibility(1.0, &player, Some("minecraft:zombie")), 1.0);
    assert_eq!(pluto_player_visibility(1.0, &player, None), 1.0, "nobody is looking");
    assert_eq!(pluto_player_visibility(0.5, &player, Some("minecraft:nan")), 0.5, "a broken answer keeps the native value");
    assert_eq!(pluto_player_visibility(0.5, &player, Some("minecraft:negative")), 0.0);
}
