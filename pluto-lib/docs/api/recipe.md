# Recipes

What a mod adds to the game's crafting: ingredient kinds of its own, and a say in what a matched grid produces. The recipe files, the vanilla recipe types, pack merging and the recipe book belong to the game; PlutoLib carries the two answers the game cannot know — what a mod-defined ingredient kind matches, and what custom data a recipe's result carries when a mod's own rules assemble it.

## Ingredient kinds

A recipe file's ingredient object whose `type` names a namespace outside `minecraft:` is a custom kind. The game asks the kind's handler twice: at load, for the item ids the kind can match (a kind that claims nothing leaves the list empty and the recipe fails, as an unknown kind does), and at craft time, for whether one concrete stack is accepted — its custom data is the usual reason to refuse. Register kinds once during startup:

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, StackView, ingredient_items, ingredient_test, register_ingredient_kind};
use serde_json::json;

register_ingredient_kind(IngredientKind {
    kind: "example:lucky",
    items: |_| vec!["minecraft:gold_ingot".to_owned()],
    test: |_, stack| stack.id == "minecraft:gold_ingot",
}).unwrap();

let data = json!({});
assert_eq!(ingredient_items("example:lucky", &data), Some(vec!["minecraft:gold_ingot".to_owned()]));
let stack = StackView { id: "minecraft:gold_ingot", count: 1, damage: 0, custom_data: None };
assert_eq!(ingredient_test("example:lucky", &data, &stack), Some(true));
assert_eq!(ingredient_test("example:unknown", &data, &stack), None);
```

## Assembly providers

When a recipe is about to be assembled from a grid, each interested provider decides: pass (the native result stands), deny (the recipe does not match this grid after all), or assemble — the result is the recipe's own template stack with its `minecraft:custom_data` component replaced by the provider's compound. Interest is decided on cheap recipe facts (form and result item) before any grid or stack is captured; any denial wins over an assembly, and the first assembly in registration order wins over later ones. Providers never see or mutate the inventory itself.

```rust
use pluto_lib::recipe_api_v1::{AssemblyDecision, AssemblyRegistration, RecipeFacts, RecipeForm, prepare_assembly, register_assembly};
use pluto_lib::{Nbt, NbtCompound};

register_assembly(AssemblyRegistration {
    id: "example:blessed",
    interest: |recipe| recipe.result_item == Some("minecraft:golden_apple"),
    callback: |_| {
        let mut data = NbtCompound::new();
        data.insert("blessed".into(), Nbt::Byte(1));
        AssemblyDecision::Assemble { custom_data: data }
    },
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1 };
assert!(prepare_assembly(&facts).is_some());
let other = RecipeFacts { result_item: Some("minecraft:apple"), ..facts };
assert!(prepare_assembly(&other).is_none());
```

## Reference

- `recipe_api_v1::register_ingredient_kind` / `ingredient_items` / `ingredient_test` / `has_ingredient_kinds` — the kind registry and its two questions.
- `recipe_api_v1::IngredientKind` — one kind: its name, its load-time item list, its craft-time stack test.
- `recipe_api_v1::StackView` — the borrowed stack facts a handler sees: item id, count, damage, and the stack's `minecraft:custom_data` compound when the adapter could supply it.
- `recipe_api_v1::register_assembly` / `prepare_assembly` / `has_assembly_providers` — the assembly registry and its interest check.
- `recipe_api_v1::AssemblyRegistration` / `AssemblyRequest` / `AssemblyDecision` / `AssemblyOutcome` — one provider, the grid request it answers, and its three answers.
- `recipe_api_v1::RecipeFacts` / `RecipeForm` / `GridFacts` — the recipe and cropped-grid facts interest and callbacks read.

The native half is source-wired (the game's ingredient dispatch and the crafting gate's assembly substitution), pure-tested at both registries, and not yet native-validated; see the support matrix in [base](base.md).
