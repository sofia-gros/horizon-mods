//! MODELED renderer owner shells. Registry/state decoding below uses the actual
//! official horizon-world crate; these shells do not prove native renderer types.
#![forbid(unsafe_code)]
extern crate self as horizon_client;
pub mod assets {
    pub struct Assets {
        pub registry: &'static horizon_world::registry::BlockRegistry,
    }
    pub struct RenderAssets {
        pub texture_layers: u32,
        pub sprite_layers: std::collections::BTreeMap<String, u32>,
        pub generation: u64,
    }
}
pub mod mesher {
    use horizon_world::fluid::FluidState;
    pub struct Info {
        pub fluid: FluidState,
        pub model: bool,
    }
    pub struct MeshTables {
        pub info: Vec<Info>,
        pub animated_layers: Vec<bool>,
    }
    impl MeshTables {
        pub fn is_animated(&self, layer: u16) -> bool {
            self.animated_layers
                .get(layer as usize)
                .copied()
                .unwrap_or(false)
        }
    }
    pub struct Mesher<'a> {
        pub assets: &'a crate::assets::Assets,
        pub render: &'a crate::assets::RenderAssets,
        pub tables: &'a MeshTables,
    }
    pub const OPAQUE_LAYER: u16 = 1 << 15;
    pub mod pluto_terrain_bindings {
        include!("terrain-bindings.rs");
    }
    pub mod pluto_terrain_binding_host {
        include!("terrain-binding-host.rs");
    }
}
mod game {
    mod pluto_fluid_render_prepare {
        include!("fluid-render-prepare.rs");
    }
    pub(crate) fn prepare(
        a: &std::sync::Arc<crate::assets::Assets>,
        r: &std::sync::Arc<crate::assets::RenderAssets>,
        t: &std::sync::Arc<crate::mesher::MeshTables>,
    ) {
        pluto_fluid_render_prepare::prepare(a, r, t);
    }
}
#[cfg(test)]
mod tests {
    include!("tests.rs");
}
