//! Tool action transforms (ToolActionTransformModule, SlimeKnights, MIT; see LICENSE): the block a tool's action turns
//! a clicked block into, for the vanilla tool actions the shipped modifiers name. The tables follow vanilla's
//! ToolModifiedState for axes, shovels and hoes; a block no rule names is left alone.
use pluto_lib::{BlockState, Identifier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolAction { AxeStrip, AxeScrape, AxeWaxOff, ShovelFlatten, Till }

impl ToolAction {
    /// The tool action named in a module (`tool_action`), None for an action this port does not transform.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "axe_strip" => Self::AxeStrip, "axe_scrape" => Self::AxeScrape, "axe_wax_off" => Self::AxeWaxOff,
            "shovel_flatten" => Self::ShovelFlatten, "till" => Self::Till,
            _ => return None,
        })
    }

    /// The name a module names this action by (the inverse of `parse`).
    pub fn name(self) -> &'static str {
        match self { Self::AxeStrip => "axe_strip", Self::AxeScrape => "axe_scrape", Self::AxeWaxOff => "axe_wax_off", Self::ShovelFlatten => "shovel_flatten", Self::Till => "till" }
    }

    /// The state the clicked block becomes, None when the action does nothing to it. Properties carry over (a log keeps its axis).
    pub fn transform(self, state: &BlockState) -> Option<BlockState> {
        if state.block.namespace() != "minecraft" { return None; }
        let path = state.block.path();
        let renamed = |name: String| Some(BlockState { block: Identifier::new("minecraft", &name).ok()?, properties: state.properties.clone() });
        let fresh = |name: &str| Some(BlockState::new(Identifier::new("minecraft", name).ok()?));
        match self {
            Self::AxeStrip => {
                if path.starts_with("stripped_") || path.starts_with("waxed_") { return None; }
                let wood = ["_log", "_wood", "_stem", "_hyphae"].iter().any(|suffix| path.ends_with(suffix)) || path == "bamboo_block";
                if wood { renamed(format!("stripped_{path}")) } else { None }
            }
            Self::AxeScrape => {
                if !path.contains("copper") || path.starts_with("waxed_") { return None; }
                if let Some(rest) = path.strip_prefix("oxidized_") { renamed(format!("weathered_{rest}")) }
                else if let Some(rest) = path.strip_prefix("weathered_") { renamed(format!("exposed_{rest}")) }
                else if let Some(rest) = path.strip_prefix("exposed_") { renamed(if rest == "copper" { "copper_block".to_owned() } else { rest.to_owned() }) }
                else { None }
            }
            Self::AxeWaxOff => {
                if !path.contains("copper") { return None; }
                let rest = path.strip_prefix("waxed_")?;
                renamed(rest.to_owned())
            }
            Self::ShovelFlatten => match path { "grass_block" | "dirt" | "podzol" | "coarse_dirt" | "mycelium" | "rooted_dirt" => fresh("dirt_path"), _ => None },
            Self::Till => match path {
                "grass_block" | "dirt" | "dirt_path" => fresh("farmland"),
                "coarse_dirt" | "rooted_dirt" => fresh("dirt"),
                _ => None,
            },
        }
    }
}

/// Whether the block above a clicked block lets a shovel or hoe work (Tinkers `require_ground`: it can be replaced).
pub(crate) fn replaceable_above(block: &Identifier) -> bool {
    block.namespace() == "minecraft" && matches!(block.path(), "air" | "cave_air" | "void_air" | "short_grass" | "tall_grass" | "fern" | "large_fern" | "dead_bush" | "snow" | "vine" | "glow_lichen")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(text: &str) -> BlockState { BlockState::parse(text).unwrap() }

    #[test]
    fn axes_strip_and_scrape_and_wax_off_by_block_name_and_keep_properties() {
        let stripped = ToolAction::AxeStrip.transform(&state("minecraft:oak_log[axis=x]")).unwrap();
        assert_eq!((stripped.block.to_string(), stripped.property("axis")), ("minecraft:stripped_oak_log".to_owned(), Some("x")));
        assert!(ToolAction::AxeStrip.transform(&state("minecraft:stripped_oak_log")).is_none());
        assert!(ToolAction::AxeStrip.transform(&state("minecraft:stone")).is_none());
        assert_eq!(ToolAction::AxeStrip.transform(&state("minecraft:bamboo_block")).unwrap().block.path(), "stripped_bamboo_block");
        assert_eq!(ToolAction::AxeScrape.transform(&state("minecraft:oxidized_cut_copper")).unwrap().block.path(), "weathered_cut_copper");
        assert_eq!(ToolAction::AxeScrape.transform(&state("minecraft:exposed_copper")).unwrap().block.path(), "copper_block");
        assert_eq!(ToolAction::AxeScrape.transform(&state("minecraft:exposed_cut_copper_stairs[facing=north]")).unwrap().block.path(), "cut_copper_stairs");
        assert!(ToolAction::AxeScrape.transform(&state("minecraft:waxed_exposed_copper")).is_none());
        assert_eq!(ToolAction::AxeWaxOff.transform(&state("minecraft:waxed_weathered_copper")).unwrap().block.path(), "weathered_copper");
        assert!(ToolAction::AxeWaxOff.transform(&state("minecraft:weathered_copper")).is_none());
    }

    #[test]
    fn shovels_flatten_and_hoes_till_the_vanilla_ground_blocks() {
        assert_eq!(ToolAction::ShovelFlatten.transform(&state("minecraft:grass_block")).unwrap().block.path(), "dirt_path");
        assert!(ToolAction::ShovelFlatten.transform(&state("minecraft:stone")).is_none());
        assert_eq!(ToolAction::Till.transform(&state("minecraft:dirt")).unwrap().block.path(), "farmland");
        assert_eq!(ToolAction::Till.transform(&state("minecraft:coarse_dirt")).unwrap().block.path(), "dirt");
        assert!(ToolAction::Till.transform(&state("minecraft:farmland")).is_none());
        assert!(ToolAction::Till.transform(&state("modded:dirt")).is_none(), "only vanilla's own blocks");
        assert!(replaceable_above(&Identifier::parse("minecraft:air").unwrap()));
        assert!(!replaceable_above(&Identifier::parse("minecraft:stone").unwrap()));
        assert_eq!(ToolAction::parse("till"), Some(ToolAction::Till));
        assert_eq!(ToolAction::parse("shield_disable"), None);
    }
}
