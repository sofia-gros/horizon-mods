// Source-specific recipe identity and actual derivation remain TConstruct-owned.
use super::{pluto_derivations as api, Recipe};
use horizon_items::items::ItemStack;
use pluto_inventory_api::crafting::products::Pair;
fn assembly_interest(recipe: &Recipe) -> bool {
    matches!(recipe, Recipe::Shapeless { result, .. }
        if result.item.as_deref() == Some("tconstruct:overslime_bottle_recipe"))
}
fn remainder_interest(recipe: &str) -> bool { recipe == "tconstruct:overslime_bottle_recipe" }
fn derive(grid: &[Option<ItemStack>]) -> Option<Pair<ItemStack>> {
    crate::tconstruct_source_overslime_crafting::plan(grid)
        .map(|(output, remainders)| Pair { output, remainders })
}
pub(super) fn entry() -> api::PureRecipe {
    api::PureRecipe { id: "tconstruct:overslime", assembly_interest, remainder_interest, derive }
}
pub(super) fn register() -> Result<(), api::RegistrationError> { api::register(entry()) }
