//! `slimeknights.tconstruct.library.materials`.
pub mod ids;
pub mod manager;
pub mod material;
pub mod registry;
pub mod stats;
pub mod traits;

pub use ids::{MaterialId, MaterialStatsId, MaterialVariantId};
pub use manager::MaterialManager;
pub use material::{Material, Rarity};
pub use registry::{MaterialRegistry, MaterialVariant};
