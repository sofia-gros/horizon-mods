use crate::crafting::Recipe;
const MARKER: &str = "tconstruct:fake_metal_conversion";
pub fn marked(recipe: &Recipe) -> bool {
    matches!(recipe, Recipe::Shaped { cells, .. } if cells.iter().flatten().any(|i|
        i.custom.as_ref().is_some_and(|(kind, _)| kind == MARKER)))
}

