// MIT License
//
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// Version-locked adapter. The copied native body and face filter are regenerated
// from exact official source by check_terrain_cell_plan.py. No consumer is
// registered by this module. GPU execution is not established by type checks.
use super::pluto_terrain_bindings as bindings;
use super::pluto_terrain_cell_geometry as geometry;
use super::pluto_terrain_cells as api;
use super::*;
use crate::models::catalog::BakedModel;
use horizon_assets::{bake::ModelQuad, model::Material};
use std::ops::ControlFlow;

pub(super) fn mesh(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    out: &mut SectionMesh,
    scratch: &mut MeshScratch,
) -> ControlFlow<SectionMesh> {
    // Freeze even an empty registry: late policy must not affect only new meshes.
    let Some(providers) = api::snapshot() else {
        return ControlFlow::Continue(());
    };
    if providers.is_empty() {
        return ControlFlow::Continue(());
    }
    let (assets, render, tables) = (mesher.assets, mesher.render, mesher.tables);
    let mut staged = None;
    let mut invalid = false;
    let checked = super::pluto_terrain_binding_host::with_material_owners(
        assets,
        render,
        tables,
        |status, materials| {
            let bindings::Status::Ready(catalog) = status else {
                return;
            };
            if catalog.by_state.is_empty() {
                return;
            }
            staged = Some(original(
                mesher,
                region,
                sy,
                std::mem::take(out),
                scratch,
                catalog,
                providers,
                materials,
                &mut invalid,
            ));
        },
    );
    match staged {
        Some(mesh) if checked.is_ok() && !invalid => ControlFlow::Break(mesh),
        Some(mesh) => {
            // Only private output was staged. Reuse its allocation for the
            // ORIGINAL native body, whose first operation clears all contents.
            // This cannot undo arbitrary provider side effects (forbidden).
            *out = mesh;
            ControlFlow::Continue(())
        }
        None => ControlFlow::Continue(()),
    }
}

struct Neighbors<'a> {
    pad: &'a Padded,
    catalog: &'a bindings::Catalog,
    pos: [i32; 3],
}
impl api::Neighborhood for Neighbors<'_> {
    fn binding(&self, offset: [i8; 3]) -> Option<&bindings::Binding> {
        if offset.iter().any(|v| !(-1..=1).contains(v)) {
            return None;
        }
        let p = std::array::from_fn::<_, 3, _>(|i| self.pos[i] + i32::from(offset[i]));
        self.catalog
            .by_state
            .get(&self.pad.blocks[pi(p[0], p[1], p[2])])
    }
}
fn direction(face: geometry::Face) -> ModelDirection {
    match face {
        geometry::Face::Up => ModelDirection::Up,
        geometry::Face::Down => ModelDirection::Down,
        geometry::Face::North => ModelDirection::North,
        geometry::Face::South => ModelDirection::South,
        geometry::Face::West => ModelDirection::West,
        geometry::Face::East => ModelDirection::East,
    }
}
enum Emission {
    Native,
    Emitted,
    Invalid,
}
// Only one selected model, at most six faces, one per direction, one resolved
// sprite. Multi-part/multi-material/ambiguous models retain native rendering.
fn selected<'a>(models: &[&'a BakedModel], sprite: u16) -> Option<[Option<&'a BakedFace>; 6]> {
    if models.len() != 1 || models[0].faces.is_empty() || models[0].faces.len() > 6 {
        return None;
    }
    let mut faces = [None; 6];
    for face in &models[0].faces {
        if face.layer != u32::from(sprite) || faces[face.quad.facing as usize].is_some() {
            return None;
        }
        faces[face.quad.facing as usize] = Some(face);
    }
    Some(faces)
}
// A material template may change only sprite-derived metadata. All native
// nontexture attributes remain the selected ordinary model's exact values.
fn compatible(source: &BakedFace, template: &BakedFace) -> bool {
    let (a, b) = (&source.quad, &template.quad);
    a.facing == b.facing
        && a.cull == b.cull
        && a.tint_index == b.tint_index
        && a.shade_direction_override == b.shade_direction_override
        && a.light_emission == b.light_emission
        && a.light == b.light
        && a.color == b.color
        && a.ambient_occlusion == b.ambient_occlusion
        && a.render_type == b.render_type
        && a.shade == b.shade
        && a.material.force_translucent == b.material.force_translucent
}
fn remap(source: &BakedFace, quad: &geometry::Quad) -> BakedFace {
    let q = &source.quad;
    BakedFace {
        quad: ModelQuad {
            positions: quad.vertices.map(|v| v.position),
            uv: quad.vertices.map(|v| v.uv),
            facing: q.facing,
            cull: q.cull,
            // The pinned Mesher::face uses only force_translucent; baked layer
            // already owns the resolved sprite. Empty String allocates nothing.
            // The complete source guard pins this assumption, not just a name.
            material: Material {
                sprite: String::new(),
                force_translucent: q.material.force_translucent,
            },
            tint_index: q.tint_index,
            shade_direction_override: q.shade_direction_override,
            light_emission: q.light_emission,
            light: q.light,
            color: q.color,
            ambient_occlusion: q.ambient_occlusion,
            render_type: q.render_type,
            shade: q.shade,
        },
        layer: source.layer,
        opaque: source.opaque,
        translucent: source.translucent,
        fusion: source.fusion,
    }
}
#[allow(clippy::too_many_arguments)]
fn emit(
    mesher: &Mesher<'_>,
    pad: &Padded,
    tints: &mut Tints,
    out: &mut SectionMesh,
    pos: [i32; 3],
    origin: [f32; 3],
    models: &[&BakedModel],
    ao: bool,
    (id, state): (u32, &Info),
    world: BlockPos,
    interior_leaf: bool,
    visible: &[bool; 6],
    catalog: &bindings::Catalog,
    providers: &std::collections::BTreeMap<String, api::RegisteredProvider>,
    budget: &mut api::Budget,
    materials: &super::pluto_terrain_binding_host::Materials,
) -> Emission {
    let Some(binding) = catalog.by_state.get(&id) else {
        return Emission::Native;
    };
    let Some(provider) = providers.get(&binding.group) else {
        return Emission::Native;
    };
    if state.fluid.kind() != FluidKind::Empty {
        return Emission::Native;
    }
    let Some(faces) = selected(models, binding.sprite) else {
        return Emission::Native;
    };
    let neighborhood = Neighbors { pad, catalog, pos };
    let cell = api::CellContext {
        binding,
        world: [world.x, world.y, world.z],
        neighborhood: &neighborhood,
    };
    let material = materials.get(&id);
    let painted = match provider {
        api::RegisteredProvider::Geometry(provider) => {
            // A geometry-only consumer must not silently ignore explicit style.
            if material.is_some() {
                return Emission::Native;
            }
            let Some(batch) = provider(&cell) else {
                return Emission::Native;
            };
            api::PaintedBatch::new(batch, api::MaterialSlot::Primary)
        }
        api::RegisteredProvider::Material(provider) => {
            let Some(material) = material else {
                return Emission::Native;
            };
            let Some(batch) = provider(&api::MaterialContext {
                cell,
                appearance: &material.facts,
            }) else {
                return Emission::Native;
            };
            batch
        }
    };
    let batch = painted.geometry();
    if !api::cell_local(batch) || !budget.quads(batch.as_slice().len()) {
        return Emission::Invalid;
    }
    // Validate every source face before any vertex emission. A resource pack
    // lacking an emitted direction cannot silently lose its existing model.
    if batch
        .as_slice()
        .iter()
        .any(|q| faces[direction(q.face) as usize].is_none())
    {
        return Emission::Invalid;
    }
    // Validate every material and its opacity proof before any custom vertex.
    for (index, quad) in batch.as_slice().iter().enumerate() {
        let source = faces[direction(quad.face) as usize].expect("validated direction");
        let selected = match painted.slot(index).expect("bounded quad") {
            api::MaterialSlot::Primary => source,
            api::MaterialSlot::Secondary => {
                let Some(material) = material else {
                    return Emission::Invalid;
                };
                let Some(template) = &material.faces[direction(quad.face) as usize] else {
                    return Emission::Invalid;
                };
                if material.ambient_occlusion != models[0].ambient_occlusion
                    || !compatible(source, template)
                {
                    return Emission::Invalid;
                }
                template
            }
        };
        if !api::opacity_allows(
            selected.opaque,
            selected.quad.render_type.is_some(),
            &selected.quad.uv,
            &quad.vertices.map(|v| v.uv),
        ) {
            return Emission::Invalid;
        }
    }
    let [x, y, z] = pos;
    let connected = &mesher.render.connected;
    for (index, quad) in batch.as_slice().iter().enumerate() {
        let source = faces[direction(quad.face) as usize].expect("validated direction");
        let mut owned = remap(source, quad);
        if painted.slot(index) == Some(api::MaterialSlot::Secondary) {
            let template = material.expect("validated material").faces
                [direction(quad.face) as usize]
                .as_ref()
                .expect("validated template");
            owned.layer = template.layer;
            owned.opaque = template.opaque;
            owned.translucent = template.translucent;
            owned.fusion = template.fusion;
        }
        if let Some(material) = material {
            owned.quad.color = api::tint_color(source.quad.color, material.facts.tint);
        }
        let face = &owned;
        let before = out.solid.len().saturating_add(out.translucent.len());
        // BEGIN EXACT NATIVE FACE FILTER
        if face.quad.cull.is_some_and(|d| !visible[d as usize]) {
            continue;
        }
        // A pane's top and bottom strips between stacked panes, with
        // connected textures.
        if state.pane
            && !connected.is_empty()
            && let Some(dy) = match face.quad.facing {
                ModelDirection::Up => Some(1),
                ModelDirection::Down => Some(-1),
                _ => None,
            }
        {
            let p = &face.quad.positions;
            // Only on the block's top or bottom plane: a pack's own pane
            // model may have ledges between.
            let plane = if dy > 0 { 1.0 } else { 0.0 };
            let on_edge = p.iter().all(|v| (v[1] - plane).abs() < 1e-4);
            let centre = [0, 2].map(|k| p.iter().map(|v| v[k]).sum::<f32>() / 4.0);
            let other = pad.blocks[pi(x, y + dy, z)];
            if on_edge && connected.stacked_pane_hides(id, other, centre) {
                continue;
            }
        }
        mesher.face(
            pad,
            tints,
            out,
            [x, y, z],
            origin,
            face,
            ao,
            (id, state),
            world,
            interior_leaf,
        );
        // END EXACT NATIVE FACE FILTER
        let after = out.solid.len().saturating_add(out.translucent.len());
        if !budget.vertices(after.saturating_sub(before)) {
            return Emission::Invalid;
        }
    }
    Emission::Emitted
}

// BEGIN EXACT NATIVE BODY WITH REVIEWED DISPATCH HUNKS
fn original(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    mut out: SectionMesh,
    scratch: &mut MeshScratch,
    bindings: &bindings::Catalog,
    providers: &std::collections::BTreeMap<String, api::RegisteredProvider>,
    materials: &super::pluto_terrain_binding_host::Materials,
    invalid: &mut bool,
) -> SectionMesh {
    let mut budget = api::Budget::default();
    out.clear();
    let center = region[1][1].expect("centre chunk");
    let section = &center.sections[sy as usize];
    if section.is_uniform(0) {
        return out;
    }
    let own = scratch.own.get_or_insert_with(|| Box::new([0; 4096]));
    scratch.padded.fill(region, sy, own);
    let pad = &scratch.padded;
    solid_rows(pad, &mesher.tables.solid, &mut scratch.solid_rows);
    let opaque = &scratch.solid_rows;
    out.visibility = visibility_rows(&open_rows(opaque));
    let base_y = center.min_y() + sy * 16;
    if mesher.leaf_culling.is_interior() {
        scratch.leaf.prepare(
            mesher.leaf_culling,
            pad,
            &mesher.tables.info,
            region,
            base_y,
        );
    }
    let mut tints = Tints::new(
        region,
        mesher.biomes,
        mesher.zoom,
        mesher.blend_radius,
        [center.x, center.z, base_y],
    );
    let info = &mesher.tables.info;
    let full_faces = &mesher.tables.full_faces;
    let catalog = &mesher.render.model_catalog;
    let mut models = Vec::new();
    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                let id = pad.blocks[pi(x, y, z)];
                if id == 0 {
                    continue;
                }
                let state = &info[id as usize];
                if state.block_entity {
                    out.block_entities
                        .push(([center.x * 16 + x, base_y + y, center.z * 16 + z], id));
                }
                if state.emission > 0 {
                    out.lights
                        .push(([center.x * 16 + x, base_y + y, center.z * 16 + z], id));
                }
                if hidden(opaque, x, y, z) {
                    continue;
                }
                if state.fluid.kind() != FluidKind::Empty {
                    let water = tints.get(Resolver::Water, x, y, z);
                    let shade = (
                        mesher.nether_shading,
                        mesher.shader_pack_lighting,
                        mesher.separate_ao,
                        mesher.shader_pack,
                    );
                    fluid::tesselate(mesher.tables, shade, pad, water, &mut out, [x, y, z]);
                }
                if !state.model {
                    continue;
                }
                let world = BlockPos::new(center.x * 16 + x, base_y + y, center.z * 16 + z);
                if catalog
                    .models_at(BlockStateId(id), world, &mut models)
                    .is_err()
                {
                    continue;
                }
                // ModelBlockRenderer: the first part decides smooth lighting
                // for all; light-emitting blocks are always lit flat.
                let Some(first) = models.first() else {
                    continue;
                };
                let ao = mesher.smooth_lighting && state.emission == 0 && first.ambient_occlusion;
                let origin = match offset(state, world.x, world.z) {
                    Some(o) => [
                        x as f32 + o[0] as f32,
                        y as f32 + o[1] as f32,
                        z as f32 + o[2] as f32,
                    ],
                    None => [x as f32, y as f32, z as f32],
                };
                let mut visible = [false; 6];
                let interior_leaf = state.leaves
                    && mesher.leaf_culling.is_interior()
                    && scratch.leaf.interior(mesher.leaf_culling, x, y, z);
                for d in Direction::ALL {
                    let [dx, dy, dz] = d.normal();
                    let neighbor = pad.blocks[pi(x + dx, y + dy, z + dz)];
                    // `should_render_face`'s first test, without its lookups: a
                    // full occlusion face toward this block hides the face.
                    if full_faces[neighbor as usize] >> d.opposite() as u8 & 1 != 0 {
                        continue;
                    }
                    let other = &info[neighbor as usize];
                    // LeavesBlock.skipRendering with opaque leaves.
                    let opaque_leaves = !mesher.cutout_leaves && state.leaves && other.leaves;
                    let culled_leaf = match mesher.leaf_culling {
                        LeafCullingMode::Hollow | LeafCullingMode::Hidden => {
                            state.leaves
                                && other.leaves
                                && leaf_culling::in_section(x + dx, y + dy, z + dz)
                                && (mesher.leaf_culling == LeafCullingMode::Hollow
                                    || leaf_culling::backed(
                                        pad,
                                        info,
                                        region,
                                        base_y,
                                        [x + 2 * dx, y + 2 * dy, z + 2 * dz],
                                    ))
                        }
                        _ => false,
                    };
                    visible[d as usize] = !opaque_leaves
                        && !culled_leaf
                        && mesher
                            .visibility
                            .should_render_face(BlockStateId(id), BlockStateId(neighbor), d, true)
                            .unwrap_or(true);
                }
                match emit(
                    mesher,
                    pad,
                    &mut tints,
                    &mut out,
                    [x, y, z],
                    origin,
                    &models,
                    ao,
                    (id, state),
                    world,
                    interior_leaf,
                    &visible,
                    bindings,
                    providers,
                    &mut budget,
                    materials,
                ) {
                    Emission::Native => {}
                    Emission::Emitted => continue,
                    Emission::Invalid => {
                        *invalid = true;
                        return out;
                    }
                }
                let connected = &mesher.render.connected;
                for model in &models {
                    for face in &model.faces {
                        if face.quad.cull.is_some_and(|d| !visible[d as usize]) {
                            continue;
                        }
                        // A pane's top and bottom strips between stacked panes, with
                        // connected textures.
                        if state.pane
                            && !connected.is_empty()
                            && let Some(dy) = match face.quad.facing {
                                ModelDirection::Up => Some(1),
                                ModelDirection::Down => Some(-1),
                                _ => None,
                            }
                        {
                            let p = &face.quad.positions;
                            // Only on the block's top or bottom plane: a pack's own pane
                            // model may have ledges between.
                            let plane = if dy > 0 { 1.0 } else { 0.0 };
                            let on_edge = p.iter().all(|v| (v[1] - plane).abs() < 1e-4);
                            let centre = [0, 2].map(|k| p.iter().map(|v| v[k]).sum::<f32>() / 4.0);
                            let other = pad.blocks[pi(x, y + dy, z)];
                            if on_edge && connected.stacked_pane_hides(id, other, centre) {
                                continue;
                            }
                        }
                        mesher.face(
                            pad,
                            &mut tints,
                            &mut out,
                            [x, y, z],
                            origin,
                            face,
                            ao,
                            (id, state),
                            world,
                            interior_leaf,
                        );
                    }
                }
            }
        }
    }
    out.finish_sprite_layers();
    out
}
// END EXACT NATIVE BODY WITH REVIEWED DISPATCH HUNKS

#[cfg(test)]
mod material_contract_tests {
    use super::*;
    fn face() -> BakedFace {
        BakedFace {
            quad: ModelQuad {
                positions: [[0.0; 3]; 4],
                uv: [[0.0; 2]; 4],
                facing: ModelDirection::Up,
                cull: None,
                material: Material {
                    sprite: "test:still".into(),
                    force_translucent: false,
                },
                tint_index: -1,
                shade_direction_override: None,
                light_emission: 0,
                light: [0; 2],
                color: None,
                ambient_occlusion: None,
                render_type: None,
                shade: true,
            },
            layer: 1,
            opaque: true,
            translucent: false,
            fusion: None,
        }
    }
    #[test]
    fn prepared_texture_metadata_can_differ_without_changing_native_attributes() {
        let source = face();
        let mut flow = face();
        flow.layer = 2;
        flow.opaque = false;
        flow.translucent = true;
        flow.fusion = std::num::NonZeroU16::new(1);
        flow.quad.material.sprite = "test:flow".into();
        flow.quad.positions = [[1.0; 3]; 4];
        flow.quad.uv = [[1.0; 2]; 4];
        assert!(compatible(&source, &flow));
    }
    #[test]
    fn every_nontexture_attribute_mismatch_rejects_the_template() {
        let source = face();
        for change in 0..11 {
            let mut template = face();
            match change {
                0 => template.quad.facing = ModelDirection::Down,
                1 => template.quad.cull = Some(ModelDirection::Up),
                2 => template.quad.tint_index = 0,
                3 => template.quad.shade_direction_override = Some(ModelDirection::North),
                4 => template.quad.light_emission = 1,
                5 => template.quad.light = [1, 0],
                6 => template.quad.color = Some(0),
                7 => template.quad.ambient_occlusion = Some(false),
                8 => template.quad.render_type = Some(horizon_assets::bake::RenderType::Solid),
                9 => template.quad.shade = false,
                _ => template.quad.material.force_translucent = true,
            }
            assert!(!compatible(&source, &template), "change {change}");
        }
    }
}
