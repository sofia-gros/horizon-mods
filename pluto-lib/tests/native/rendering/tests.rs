use super::*;
use horizon_client::hud::{Blend, HudVertex};

fn rect() -> Rect {
    Rect {
        u0: 0.013,
        v0: 0.127,
        u1: 0.917,
        v1: 0.603,
        w: 37,
        h: 83,
    }
}
fn batch() -> HudBatch {
    let mut b = HudBatch::new([640, 480]);
    b.rect_uv(-7.0, 3.0, 2.0, 9.0, rect(), 0x12345678);
    b.blends.push((0..6, Blend::Invert));
    b.glint_alpha = 0.125;
    b.blur_at = 3;
    b
}
fn hud_equal(a: &[HudVertex], b: &[HudVertex]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.pos.map(f32::to_bits), b.pos.map(f32::to_bits));
        assert_eq!(a.uv.map(f32::to_bits), b.uv.map(f32::to_bits));
        assert_eq!(a.color, b.color);
        assert_eq!(a.depth.to_bits(), b.depth.to_bits());
        assert_eq!(a.glint_uv.map(f32::to_bits), b.glint_uv.map(f32::to_bits));
        assert_eq!(a.text, b.text);
    }
}
fn mesh_equal(a: &[Vertex], b: &[Vertex]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.pos.map(f32::to_bits), b.pos.map(f32::to_bits));
        assert_eq!(a.uv.map(f32::to_bits), b.uv.map(f32::to_bits));
        assert_eq!(a.layer, b.layer);
        assert_eq!(a.color, b.color);
        assert_eq!(a.light, b.light);
        assert_eq!(a.normal, b.normal);
        assert_eq!(a.glint_uv.map(f32::to_bits), b.glint_uv.map(f32::to_bits));
    }
}
#[test]
fn tiled_vertices_match_frozen_float32_oracle() {
    for width in [12.0, 16.0, 52.0, 106.0] {
        for height in [
            0.0, 1.0, 15.0, 16.0, 17.0, 36.0, 52.0, 90.0, 106.0, 16.25, 51.75,
        ] {
            for [ox, oy, layer] in [[0.0, 0.0, 0.0], [-171.125, 37.4, 11.375]] {
                let r = rect();
                let mut old = batch();
                let mut new = batch();
                oracle::tiles(&mut old, r, ox, oy, layer, width, height, 0x35729acf);
                tiled_bottom_up(
                    &mut new,
                    Sprite(&r),
                    [ox + 8.0, oy + 122.0 - layer, width, height],
                    [16.0; 2],
                    0x35729acf,
                    64,
                )
                .unwrap();
                hud_equal(&old.vertices, &new.vertices);
                assert_eq!(old.blends, new.blends);
                assert_eq!(old.glint_alpha, new.glint_alpha);
                assert_eq!(old.blur_at, new.blur_at);
                assert_eq!(old.patches.len(), new.patches.len());
            }
        }
    }
}
#[test]
fn single_column_callers_match_without_added_offset_rounding() {
    for fuel in [false, true] {
        for height in [0.0, 0.25, 16.0, 17.5, 36.0, 90.0] {
            let r = rect();
            let (ox, oy) = (-19.125, 6.75);
            let mut old = batch();
            let mut new = batch();
            oracle::narrow(&mut old, r, ox, oy, height, 0x12345678, fuel);
            let (left, bottom, width) = if fuel {
                (ox + 152.0, oy + 122.0, 16.0)
            } else {
                (ox + 153.0, oy + 68.0, 12.0)
            };
            tiled_bottom_up(
                &mut new,
                Sprite(&r),
                [left, bottom, width, height],
                [16.0; 2],
                0x12345678,
                64,
            )
            .unwrap();
            hud_equal(&old.vertices, &new.vertices);
        }
    }
}
#[test]
fn crop_preserves_mask_extent_even_with_non_16_logical_sprite() {
    let r = rect();
    let mut old = batch();
    let mut new = batch();
    let (x, y) = (-4.25, 17.125);
    oracle::mask(&mut old, r, x, y, 0x12789abc);
    for (left, top, width) in [(6., 3., 4.), (5., 4., 6.), (5., 5., 6.), (6., 6., 4.)] {
        crop(
            &mut new,
            Sprite(&r),
            [left, top, width, 1.0],
            [16.0; 2],
            [x + left, y + top, width, 1.0],
            0x12789abc,
        )
        .unwrap();
    }
    hud_equal(&old.vertices, &new.vertices);
}
#[test]
fn cuboid_vertices_match_frozen_oracle_including_zero_height_and_prefix() {
    let r = rect();
    // Machine, table, basin, downward stream and side-facing stream bounds.
    for bounds in [
        [0.08, 8.08, 0.08, 15.92, 15.92, 15.92],
        [1., 15., 1., 15., 15.9, 15.],
        [2.1, 4., 2.1, 13.9, 15.9, 13.9],
        [6., 0., 6., 10., 16., 10.],
        [6., 6., 10., 10., 9., 16.],
        [6., 0., 8., 10., 9., 10.],
        [6., 6., 0., 10., 9., 6.],
        [6., 0., 6., 10., 9., 8.],
        [0., 6., 6., 6., 9., 10.],
        [6., 0., 6., 8., 9., 10.],
        [10., 6., 6., 16., 9., 10.],
        [8., 0., 6., 10., 9., 10.],
    ] {
        for fraction in [0.0, 0.125, 0.5, 1.0] {
            for sides in [false, true] {
                let mut b = bounds.map(|v| v / 16.0);
                b[4] = b[1] + (b[4] - b[1]) * fraction;
                let origin = (glam::DVec3::new(-23., 51., 9.)
                    - glam::DVec3::new(8.25, 64.125, -8.75))
                .as_vec3();
                let seed = Vertex {
                    pos: [1.0; 3],
                    uv: [0.5; 2],
                    layer: 7,
                    color: 123,
                    light: 256,
                    normal: [1, 2, 3, 4],
                    glint_uv: [0.1; 2],
                };
                let mut old = vec![seed];
                let mut new = vec![seed];
                oracle::mesh(&mut old, r, origin, b, sides, 0x6789abcd, 0x80d0);
                cuboid(
                    &mut new,
                    Sprite(&r),
                    origin.to_array(),
                    b,
                    if sides { TOP_AND_SIDES } else { TOP },
                    [16.0; 2],
                    0x6789abcd,
                    0x80d0,
                )
                .unwrap();
                mesh_equal(&old, &new);
            }
        }
    }
}
#[test]
fn face_selection_keeps_order_and_all_fields() {
    let r = rect();
    let mut all = vec![];
    cuboid(
        &mut all,
        Sprite(&r),
        [0.0; 3],
        [0., 0., 0., 1., 1., 1.],
        ALL_FACES,
        [16.0; 2],
        7,
        9,
    )
    .unwrap();
    assert_eq!(all.len(), 24);
    for (index, face) in ALL_FACES.iter().enumerate() {
        let mut one = vec![];
        cuboid(
            &mut one,
            Sprite(&r),
            [0.0; 3],
            [0., 0., 0., 1., 1., 1.],
            &[*face],
            [16.0; 2],
            7,
            9,
        )
        .unwrap();
        mesh_equal(&all[index * 4..index * 4 + 4], &one);
    }
    assert_eq!(all[20].pos, [0., 0., 1.]);
    assert_eq!(all[23].pos, [0., 0., 0.]);
}
#[test]
fn every_packed_light_and_legal_minimum_matches_old_formula() {
    for packed in 0..=u8::MAX {
        for minimum in 0..=15 {
            assert_eq!(
                minimum_block_light(packed, minimum),
                Ok(oracle::light(packed, minimum))
            );
        }
    }
    assert_eq!(minimum_block_light(0, 16), Err(DrawError::InvalidInput));
}
#[test]
fn invalid_and_over_budget_tiling_never_appends() {
    let r = rect();
    let before = batch();
    for (destination, tile, budget) in [
        ([0., 0., 106., 106.], [16.; 2], 48),
        ([0., 0., f32::INFINITY, 1.], [16.; 2], 64),
        ([0., 0., 1., 1.], [0., 16.], 64),
        ([0., 0., -1., 1.], [16.; 2], 64),
        ([0., 0., 1., 1.], [f32::from_bits(1), 1.], 64),
        ([0., 0., 10000., 10000.], [1.; 2], usize::MAX),
        ([f32::MAX, 0., f32::MAX, 16.], [16.; 2], 64),
        ([0., f32::NAN, 1., 1.], [16.; 2], 64),
    ] {
        let mut b = batch();
        assert!(tiled_bottom_up(&mut b, Sprite(&r), destination, tile, 7, budget).is_err());
        hud_equal(&before.vertices, &b.vertices);
        assert_eq!(before.blends, b.blends);
    }
    let mut b = batch();
    tiled_bottom_up(&mut b, Sprite(&r), [0., 0., 0., 1.], [16.; 2], 7, 0).unwrap();
    hud_equal(&before.vertices, &b.vertices);
}
#[test]
fn invalid_crop_and_mesh_are_atomic() {
    let r = rect();
    let before = batch();
    for (source, extent, destination) in [
        ([0., 0., 1., 1.], [0., 16.], [0., 0., 1., 1.]),
        ([0., f32::NAN, 1., 1.], [16.; 2], [0., 0., 1., 1.]),
        ([0., 0., 1., 1.], [16.; 2], [0., 0., -1., 1.]),
    ] {
        let mut b = batch();
        assert!(crop(&mut b, Sprite(&r), source, extent, destination, 7).is_err());
        hud_equal(&before.vertices, &b.vertices);
    }
    let mut b = batch();
    crop(
        &mut b,
        Sprite(&r),
        [0., 0., 1., 1.],
        [16.; 2],
        [0., 0., 0., 1.],
        7,
    )
    .unwrap();
    hud_equal(&before.vertices, &b.vertices);
    for bounds in [
        [1., 0., 0., 0., 1., 1.],
        [0., 0., 0., f32::INFINITY, 1., 1.],
    ] {
        let mut out = vec![];
        assert!(cuboid(&mut out, Sprite(&r), [0.; 3], bounds, TOP, [16.; 2], 7, 9).is_err());
        assert!(out.is_empty());
    }
    let mut out = vec![];
    assert!(
        cuboid(
            &mut out,
            Sprite(&r),
            [f32::MAX; 3],
            [0., 0., 0., f32::MAX, f32::MAX, f32::MAX],
            ALL_FACES,
            [16.; 2],
            7,
            9
        )
        .is_err()
    );
    assert!(out.is_empty());
}

#[test]
fn heat_and_fire_crops_match_frozen_bodies() {
    for logical in [[256, 256], [193, 387]] {
        let r = Rect {
            w: logical[0],
            h: logical[1],
            ..rect()
        };
        for [x, y] in [[0.0, 0.0], [-23.125, 74.375]] {
            for sx in [176.0, 179.0, 182.0, 185.0] {
                for height in [0.0, 0.125, 1.0, 7.75, 15.0, 16.0] {
                    let mut old = batch();
                    let mut new = batch();
                    crop_oracle::heat(&mut old, r, x, y, sx, height);
                    if height > 0.0 {
                        crop(
                            &mut new,
                            Sprite(&r),
                            [sx, 150.0 + 16.0 - height, 3.0, height],
                            Sprite(&r).logical_size(),
                            [x - 4.0, y + 16.0 - height, 3.0, height],
                            u32::MAX,
                        )
                        .unwrap();
                    }
                    hud_equal(&old.vertices, &new.vertices);
                    assert_eq!(old.blends, new.blends);
                }
            }
            for height in 0..=14 {
                let height = height as f32;
                let mut old = batch();
                let mut new = batch();
                crop_oracle::fire(&mut old, r, x, y, height);
                if height != 0.0 {
                    crop(
                        &mut new,
                        Sprite(&r),
                        [176.0, 136.0 + 14.0 - height, 14.0, height],
                        Sprite(&r).logical_size(),
                        [x, y + 14.0 - height, 14.0, height],
                        u32::MAX,
                    )
                    .unwrap();
                }
                hud_equal(&old.vertices, &new.vertices);
                assert_eq!(old.blends, new.blends);
            }
        }
    }
}

#[test]
fn panel_and_both_gauges_match_frozen_bodies() {
    for logical in [[256, 256], [193, 387]] {
        let r = Rect {
            w: logical[0],
            h: logical[1],
            ..rect()
        };
        for [ox, oy] in [[0.0, 0.0], [-23.125, 74.375]] {
            for has_heater in [false, true] {
                let mut old = batch();
                let mut new = batch();
                crop_oracle::panel(&mut old, r, ox, oy, has_heater);
                let (x, width, height, sx) = if has_heater {
                    (150.0, 18.0, 36.0, 176.0)
                } else {
                    (152.0, 14.0, 38.0, 194.0)
                };
                crop(
                    &mut new,
                    Sprite(&r),
                    [sx, 52.0, width, height],
                    Sprite(&r).logical_size(),
                    [ox + x, oy + 31.0, width, height],
                    u32::MAX,
                )
                .unwrap();
                hud_equal(&old.vertices, &new.vertices);
            }
            let mut old = batch();
            let mut new = batch();
            crop_oracle::melter_gauge(&mut old, r, ox, oy);
            crop(
                &mut new,
                Sprite(&r),
                [176.0, 0.0, 52.0, 52.0],
                Sprite(&r).logical_size(),
                [ox + 90.0, oy + 16.0, 52.0, 52.0],
                u32::MAX,
            )
            .unwrap();
            hud_equal(&old.vertices, &new.vertices);
            let mut old = batch();
            let mut new = batch();
            crop_oracle::smeltery_gauge(&mut old, r, ox, oy);
            crop(
                &mut new,
                Sprite(&r),
                [176.0, 0.0, 80.0, 106.0],
                Sprite(&r).logical_size(),
                [ox + 8.0, oy + 16.0, 80.0, 106.0],
                u32::MAX,
            )
            .unwrap();
            hud_equal(&old.vertices, &new.vertices);
        }
    }
}

#[test]
fn machine_panel_mode_hover_and_missing_sprites_match() {
    for available in 0..4 {
        let mut regions = std::collections::HashMap::new();
        if available & 1 != 0 {
            regions.insert(
                "gui/tconstruct_heating_structure".to_owned(),
                Rect {
                    w: 256,
                    h: 256,
                    ..rect()
                },
            );
        }
        if available & 2 != 0 {
            regions.insert(
                "gui/tconstruct_generic".to_owned(),
                Rect {
                    w: 64,
                    h: 128,
                    ..rect()
                },
            );
        }
        for mode in 0..=2 {
            for empty in [false, true] {
                for hovered in [false, true] {
                    for [ox, oy] in [[0.0, 0.0], [-23.125, 74.375]] {
                        let mut old = batch();
                        let mut new = batch();
                        panel_oracle::old_panel(
                            &mut old, &regions, ox, oy, mode, 1, 0, empty, hovered,
                        );
                        panel_oracle::new_panel(
                            &mut new, &regions, ox, oy, mode, 1, 0, empty, hovered,
                        );
                        hud_equal(&old.vertices, &new.vertices);
                        assert_eq!(old.blends, new.blends);
                    }
                }
            }
        }
    }
}

#[test]
fn machine_panel_paging_regions_and_padding_match() {
    for logical in [[256, 256], [193, 387]] {
        let mut regions = std::collections::HashMap::new();
        regions.insert(
            "gui/tconstruct_heating_structure".to_owned(),
            Rect {
                w: logical[0],
                h: logical[1],
                ..rect()
            },
        );
        regions.insert(
            "gui/tconstruct_generic".to_owned(),
            Rect {
                w: logical[1],
                h: logical[0],
                ..rect()
            },
        );
        for count in [0, 1, 7, 8, 14, 28, 29, 40, 41, 71, 512] {
            for offset in [0, 1, 8, 999] {
                for [ox, oy] in [[0.0, 0.0], [-23.125, 74.375]] {
                    let mut old = batch();
                    let mut new = batch();
                    panel_oracle::old_panel(
                        &mut old, &regions, ox, oy, 2, count, offset, true, true,
                    );
                    panel_oracle::new_panel(
                        &mut new, &regions, ox, oy, 2, count, offset, true, true,
                    );
                    hud_equal(&old.vertices, &new.vertices);
                    assert_eq!(old.blends, new.blends);
                }
            }
        }
    }
}

#[test]
fn station_fixed_crops_selection_scroll_and_missing_sprites_match() {
    type Draw =
        fn(&mut HudBatch, &std::collections::HashMap<String, Rect>, &str, station_oracle::Inputs);
    let pairs: [(Draw, Draw); 6] = [
        (station_oracle::old_worktable, station_oracle::new_worktable),
        (station_oracle::old_cover, station_oracle::new_cover),
        (
            station_oracle::old_name_field,
            station_oracle::new_name_field,
        ),
        (station_oracle::old_button, station_oracle::new_button),
        (
            station_oracle::old_empty_icons,
            station_oracle::new_empty_icons,
        ),
        (station_oracle::old_scroll, station_oracle::new_scroll),
    ];
    for available in 0..8 {
        for logical in [[256, 256], [193, 387]] {
            let mut regions = std::collections::HashMap::new();
            for (bit, key) in [
                (1, "gui/tconstruct_worktable"),
                (2, "gui/tconstruct_icons"),
                (4, "texture"),
            ] {
                if available & bit != 0 {
                    regions.insert(
                        key.to_owned(),
                        Rect {
                            w: logical[0],
                            h: logical[1],
                            ..rect()
                        },
                    );
                }
            }
            for state in 0..4 {
                for progress in [-1.0, 0.0, 0.125, 0.5, 1.0, 2.0, f32::NAN] {
                    let input = station_oracle::Inputs {
                        ox: -23.125,
                        oy: 74.375,
                        source: [0.; 4],
                        destination: [17.25, 41.375, 18., 18.],
                        color: 0x35729acf,
                        selected: state & 1 != 0,
                        hovered: state & 2 != 0,
                        scrollable: state & 1 != 0,
                        progress,
                        empty: [state & 1 != 0, state & 2 != 0],
                    };
                    for (old_draw, new_draw) in pairs {
                        let mut old = batch();
                        let mut new = batch();
                        old_draw(&mut old, &regions, "texture", input);
                        new_draw(&mut new, &regions, "texture", input);
                        hud_equal(&old.vertices, &new.vertices);
                        assert_eq!(old.blends, new.blends);
                    }
                }
            }
        }
    }
}

#[test]
fn station_generic_parts_beams_decorations_and_alpha_match() {
    for logical in [[256, 256], [193, 387]] {
        let mut regions = std::collections::HashMap::new();
        regions.insert(
            "texture".to_owned(),
            Rect {
                w: logical[0],
                h: logical[1],
                ..rect()
            },
        );
        for source in [
            [176., 0., 18., 18.],
            [194., 0., 18., 18.],
            [176., 15., 18., 18.],
            [176., 33., 18., 18.],
            [176., 51., 18., 18.],
            [0., 202., 2., 7.],
            [2., 202., 129., 7.],
            [131., 202., 2., 7.],
            [0., 209., 2., 7.],
            [2., 209., 129., 7.],
            [131., 209., 2., 7.],
            [0., 196., 5., 4.],
            [9., 196., 9., 4.],
            [36., 196., 5., 4.],
            [45., 196., 9., 4.],
            [0., 198., 18., 2.],
            [36., 198., 18., 2.],
        ] {
            for width in [18.0, 126.0, 128.0, 200.0] {
                for color in [u32::MAX, 0x47ffffff, 0xd1ffffff] {
                    let input = station_oracle::Inputs {
                        ox: -23.125,
                        oy: 74.375,
                        source,
                        destination: [-132., 9., width, 7.],
                        color,
                        selected: false,
                        hovered: false,
                        scrollable: false,
                        progress: 0.,
                        empty: [false; 2],
                    };
                    let mut old = batch();
                    let mut new = batch();
                    station_oracle::old_part(&mut old, &regions, "texture", input);
                    station_oracle::new_part(&mut new, &regions, "texture", input);
                    hud_equal(&old.vertices, &new.vertices);
                    let mut old = batch();
                    let mut new = batch();
                    station_oracle::old_region(&mut old, &regions, "texture", input);
                    station_oracle::new_region(&mut new, &regions, "texture", input);
                    hud_equal(&old.vertices, &new.vertices);
                }
            }
        }
    }
}

#[test]
fn side_inventory_rows_padding_scroll_and_missing_sprites_match() {
    for available in [false, true] {
        for logical in [[64, 64], [193, 387]] {
            let mut regions = std::collections::HashMap::new();
            if available {
                regions.insert(
                    "gui/tconstruct_generic".to_owned(),
                    Rect {
                        w: logical[0],
                        h: logical[1],
                        ..rect()
                    },
                );
            }
            for count in [0, 1, 5, 9, 24, 27, 28, 48, 49, 54, 71, 512] {
                for offset in [0, 1, 8, 999] {
                    for [ox, oy] in [[0.0, 0.0], [-23.125, 74.375]] {
                        let mut old = batch();
                        let mut new = batch();
                        side_oracle::old_side(&mut old, &regions, ox, oy, count, offset);
                        side_oracle::new_side(&mut new, &regions, ox, oy, count, offset);
                        hud_equal(&old.vertices, &new.vertices);
                        assert_eq!(old.blends, new.blends);
                    }
                }
            }
        }
    }
}

#[test]
fn information_panel_edges_and_missing_sprite_match() {
    for available in [false, true] {
        for logical in [[126, 83], [193, 387]] {
            let mut regions = std::collections::HashMap::new();
            if available {
                regions.insert(
                    "gui/tconstruct_panel".to_owned(),
                    Rect {
                        w: logical[0],
                        h: logical[1],
                        ..rect()
                    },
                );
            }
            for width in [80.0, 80.125, 96.0, 125.75, 126.0] {
                for height in [83.0, 184.0] {
                    for [ox, oy] in [[0.0, 0.0], [-23.125, 74.375]] {
                        let mut old = batch();
                        let mut new = batch();
                        side_oracle::old_info(&mut old, &regions, ox, oy, width, height);
                        side_oracle::new_info(&mut new, &regions, ox, oy, width, height);
                        hud_equal(&old.vertices, &new.vertices);
                        assert_eq!(old.blends, new.blends);
                    }
                }
            }
        }
    }
}
