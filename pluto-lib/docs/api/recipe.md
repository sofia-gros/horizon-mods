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

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
assert!(prepare_assembly(&facts).is_some());
let other = RecipeFacts { result_item: Some("minecraft:apple"), ..facts };
assert!(prepare_assembly(&other).is_none());
```

## Admission policies

Before a recipe is matched or assembled, each interested admission policy decides: pass (the native result stands) or deny (the recipe does not match this grid after all). A policy never assembles and never sees or mutates the inventory; interest is decided on the same cheap recipe facts, which name the recipe's loaded `id`, so a policy can admit by identity — the shape a config-gated recipe needs, since the game's data conditions cannot name a mod's config values. Any denial wins, and the first denying policy in registration order is the one reported.

```rust
use pluto_lib::recipe_api_v1::{AdmissionDecision, AdmissionRegistration, RecipeFacts, RecipeForm, prepare_admission, register_admission};

register_admission(AdmissionRegistration {
    id: "example:config_gate",
    interest: |recipe| recipe.id == Some("example:gated"),
    callback: |request| if request.recipe.id == Some("example:gated") { AdmissionDecision::Deny } else { AdmissionDecision::Pass },
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
assert!(prepare_admission(&facts).is_some());
let other = RecipeFacts { id: Some("example:other"), ..facts };
assert!(prepare_admission(&other).is_none());
```

## Reference

- `recipe_api_v1::register_ingredient_kind` / `ingredient_items` / `ingredient_test` / `has_ingredient_kinds` — the kind registry and its two questions.
- `recipe_api_v1::IngredientKind` — one kind: its name, its load-time item list, its craft-time stack test.
- `recipe_api_v1::StackView` — the borrowed stack facts a handler sees: item id, count, damage, and the stack's `minecraft:custom_data` compound when the adapter could supply it.
- `recipe_api_v1::register_assembly` / `prepare_assembly` / `has_assembly_providers` — the assembly registry and its interest check.
- `recipe_api_v1::AssemblyRegistration` / `AssemblyRequest` / `AssemblyDecision` / `AssemblyOutcome` — one provider, the grid request it answers, and its three answers.
- `recipe_api_v1::register_admission` / `prepare_admission` / `has_admission_providers` — the admission registry and its interest check.
- `recipe_api_v1::AdmissionRegistration` / `AdmissionRequest` / `AdmissionDecision` / `AdmissionOutcome` — one policy, the grid request it answers, and its two answers.
- `recipe_api_v1::RecipeFacts` / `RecipeForm` / `GridFacts` — the recipe and cropped-grid facts interest and callbacks read.

The native half is source-wired (the game's ingredient dispatch and the crafting gate's assembly substitution), pure-tested at both registries, and not yet native-validated; see the support matrix in [base](base.md).

### `StackView`

The borrowed stack facts a handler sees: `id` is the item id, `count` how many, `damage` the stack's damage, and `custom_data` the stack's `minecraft:custom_data` compound when the adapter could supply it (`None` means unavailable, not empty).

```rust
use pluto_lib::recipe_api_v1::StackView;

let stack = StackView { id: "minecraft:gold_ingot", count: 3, damage: 0, custom_data: None };
assert_eq!(stack.id, "minecraft:gold_ingot");
assert_eq!((stack.count, stack.damage), (3, 0));
assert!(stack.custom_data.is_none());
```

### `RegistrationError`

Why a registration was refused: `EmptyId` for an empty kind or provider id, and `DuplicateId` when that id is already registered; a refused registration never replaces the earlier one.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, RegistrationError, register_ingredient_kind};

let empty = IngredientKind { kind: "", items: |_| vec![], test: |_, _| true };
assert_eq!(register_ingredient_kind(empty), Err(RegistrationError::EmptyId));
let kind = IngredientKind { kind: "example:dup", items: |_| vec![], test: |_, _| true };
register_ingredient_kind(kind).unwrap();
assert_eq!(register_ingredient_kind(kind), Err(RegistrationError::DuplicateId));
```

### `IngredientKind`

One mod-defined ingredient kind: `kind` is its name, `items` runs at recipe load and names every item id the kind can match, and `test` runs at craft time against one concrete stack.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, StackView};
use serde_json::json;

let kind = IngredientKind {
    kind: "example:lucky",
    items: |_| vec!["minecraft:gold_ingot".to_owned()],
    test: |_, stack| stack.id == "minecraft:gold_ingot",
};
assert_eq!(kind.kind, "example:lucky");
assert_eq!((kind.items)(&json!({})), vec!["minecraft:gold_ingot".to_owned()]);
let stack = StackView { id: "minecraft:gold_ingot", count: 1, damage: 0, custom_data: None };
assert!((kind.test)(&json!({}), &stack));
```

### `register_ingredient_kind`

`fn register_ingredient_kind(entry: IngredientKind) -> Result<(), RegistrationError>`. Registers one ingredient kind once during startup; there is no unregister path.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, ingredient_items, register_ingredient_kind};
use serde_json::json;

register_ingredient_kind(IngredientKind {
    kind: "example:registered",
    items: |_| vec!["minecraft:gold_ingot".to_owned()],
    test: |_, stack| stack.id == "minecraft:gold_ingot",
}).unwrap();
assert_eq!(ingredient_items("example:registered", &json!({})), Some(vec!["minecraft:gold_ingot".to_owned()]));
```

### `has_ingredient_kinds`

`fn has_ingredient_kinds() -> bool`. Whether any ingredient kind has been registered.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, has_ingredient_kinds, register_ingredient_kind};

assert!(!has_ingredient_kinds());
register_ingredient_kind(IngredientKind {
    kind: "example:counted",
    items: |_| vec![],
    test: |_, _| true,
}).unwrap();
assert!(has_ingredient_kinds());
```

### `ingredient_items`

`fn ingredient_items(kind: &str, data: &Value) -> Option<Vec<String>>`. The load-time item list of a claimed kind; `None` when no handler claims the kind.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, ingredient_items, register_ingredient_kind};
use serde_json::json;

register_ingredient_kind(IngredientKind {
    kind: "example:lucky",
    items: |_| vec!["minecraft:gold_ingot".to_owned()],
    test: |_, stack| stack.id == "minecraft:gold_ingot",
}).unwrap();

let data = json!({});
assert_eq!(ingredient_items("example:lucky", &data), Some(vec!["minecraft:gold_ingot".to_owned()]));
assert_eq!(ingredient_items("example:unknown", &data), None);
```

### `ingredient_test`

`fn ingredient_test(kind: &str, data: &Value, stack: &StackView<'_>) -> Option<bool>`. The craft-time answer of a claimed kind; `None` when no handler claims the kind.

```rust
use pluto_lib::recipe_api_v1::{IngredientKind, StackView, ingredient_test, register_ingredient_kind};
use serde_json::json;

register_ingredient_kind(IngredientKind {
    kind: "example:lucky",
    items: |_| vec!["minecraft:gold_ingot".to_owned()],
    test: |_, stack| stack.id == "minecraft:gold_ingot",
}).unwrap();

let data = json!({});
let stack = StackView { id: "minecraft:gold_ingot", count: 1, damage: 0, custom_data: None };
assert_eq!(ingredient_test("example:lucky", &data, &stack), Some(true));
assert_eq!(ingredient_test("example:unknown", &data, &stack), None);
```

### `RecipeForm`

The form of the recipe being assembled, as the engine parsed it: `Shaped` with a `width` and `height`, `Shapeless`, or `Special` (a special recipe: no fixed ingredient list or result template).

```rust
use pluto_lib::recipe_api_v1::RecipeForm;

let shaped = RecipeForm::Shaped { width: 2, height: 1 };
assert_eq!(shaped, RecipeForm::Shaped { width: 2, height: 1 });
assert_ne!(shaped, RecipeForm::Shapeless);
assert_ne!(RecipeForm::Special, RecipeForm::Shapeless);
```

### `RecipeFacts`

Cheap borrowed facts about one recipe, for assembly interest checks: `form` is its `RecipeForm`, `result_item` the fixed result template's item (none for special recipes), `result_count` how many the template produces, and `id` the recipe's loaded id (`<namespace>:<path>` for a content mod's recipe, the bare name for one of vanilla's) when the adapter could name it, or none when it could not.

```rust
use pluto_lib::recipe_api_v1::{RecipeFacts, RecipeForm};

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1, id: Some("minecraft:golden_apple") };
assert_eq!(facts.form, RecipeForm::Shaped { width: 1, height: 1 });
assert_eq!(facts.result_item, Some("minecraft:golden_apple"));
assert_eq!(facts.result_count, 1);
assert_eq!(facts.id, Some("minecraft:golden_apple"));
```

### `GridFacts`

The crafting grid as the engine cropped it to the recipe's bounds: `width` and `height` are the cropped size, `left` and `top` the crop's offset in the full grid, and `cells` the row-major cells including empty positions.

```rust
use pluto_lib::recipe_api_v1::{GridFacts, StackView};

let cell = StackView { id: "minecraft:gold_ingot", count: 1, damage: 0, custom_data: None };
let cells = [Some(cell), None];
let grid = GridFacts { width: 2, height: 1, left: 1, top: 0, cells: &cells };
assert_eq!((grid.width, grid.height, grid.left, grid.top), (2, 1, 1, 0));
assert_eq!(grid.cells.len(), 2);
assert_eq!(grid.cells[0].unwrap().id, "minecraft:gold_ingot");
assert!(grid.cells[1].is_none());
```

### `AssemblyRequest`

What an assembly callback answers: `recipe` is the recipe's `RecipeFacts`, `grid` the cropped grid's `GridFacts`, and `result` the result template's native created stack, if any (not a proposed output).

```rust
use pluto_lib::recipe_api_v1::{AssemblyRequest, GridFacts, RecipeFacts, RecipeForm};

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: None };
let request = AssemblyRequest {
    recipe: &facts,
    grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] },
    result: None,
};
assert_eq!(request.recipe.result_item, Some("minecraft:apple"));
assert_eq!(request.grid.width, 0);
assert!(request.result.is_none());
```

### `AssemblyDecision`

What one assembly provider decides for one grid: `Pass` (the native result stands), `Deny` (the recipe does not match this grid after all), or `Assemble` with a `custom_data` compound that replaces the result's `minecraft:custom_data` component.

```rust
use pluto_lib::recipe_api_v1::AssemblyDecision;
use pluto_lib::{Nbt, NbtCompound};

assert_eq!(AssemblyDecision::Pass, AssemblyDecision::Pass);
assert_ne!(AssemblyDecision::Deny, AssemblyDecision::Pass);
let mut data = NbtCompound::new();
data.insert("blessed".into(), Nbt::Byte(1));
let decision = AssemblyDecision::Assemble { custom_data: data };
assert!(matches!(decision, AssemblyDecision::Assemble { .. }));
```

### `AssemblyInterest`

`type AssemblyInterest = for<'a> fn(&RecipeFacts<'a>) -> bool`. The cheap interest check of an assembly provider, decided on recipe facts before any grid or stack is captured.

```rust
use pluto_lib::recipe_api_v1::{AssemblyInterest, RecipeFacts, RecipeForm};

fn interested(recipe: &RecipeFacts<'_>) -> bool { recipe.result_item == Some("minecraft:golden_apple") }
let interest: AssemblyInterest = interested;
let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
assert!(interest(&facts));
```

### `AssemblyCallback`

`type AssemblyCallback = for<'a> fn(&AssemblyRequest<'a>) -> AssemblyDecision`. The callback an interested assembly provider runs for one grid request.

```rust
use pluto_lib::recipe_api_v1::{AssemblyCallback, AssemblyDecision, AssemblyRequest, GridFacts, RecipeFacts, RecipeForm};

fn pass(_: &AssemblyRequest<'_>) -> AssemblyDecision { AssemblyDecision::Pass }
let callback: AssemblyCallback = pass;
let facts = RecipeFacts { form: RecipeForm::Special, result_item: None, result_count: 1, id: None };
let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
assert_eq!(callback(&request), AssemblyDecision::Pass);
```

### `AssemblyRegistration`

One assembly provider to register: `id` is its registration id, `interest` its `AssemblyInterest` check, and `callback` its `AssemblyCallback`.

```rust
use pluto_lib::recipe_api_v1::{AssemblyRegistration, RecipeFacts, RecipeForm};

let registration = AssemblyRegistration {
    id: "example:blessed",
    interest: |recipe| recipe.result_item == Some("minecraft:golden_apple"),
    callback: |_| pluto_lib::recipe_api_v1::AssemblyDecision::Pass,
};
assert_eq!(registration.id, "example:blessed");
let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
assert!((registration.interest)(&facts));
```

### `AssemblyOutcome`

The result of evaluating the interested providers for one request: `Native` (no provider assembled or denied), `Denied` naming the `first` denying provider, or `Assembled` naming the `owner` provider and its `custom_data`.

```rust
use pluto_lib::recipe_api_v1::AssemblyOutcome;
use pluto_lib::{Nbt, NbtCompound};

assert_eq!(AssemblyOutcome::Native, AssemblyOutcome::Native);
assert_eq!(AssemblyOutcome::Denied { first: "example:veto" }, AssemblyOutcome::Denied { first: "example:veto" });
let mut data = NbtCompound::new();
data.insert("made".into(), Nbt::Byte(1));
let assembled = AssemblyOutcome::Assembled { owner: "example:maker", custom_data: data };
assert!(matches!(assembled, AssemblyOutcome::Assembled { owner: "example:maker", .. }));
```

### `AssemblyEvaluation`

An ephemeral snapshot of the interested providers' callbacks, returned by `prepare_assembly`; the adapter evaluates it synchronously with the same recipe facts. It is not a recipe or world identity or authority token.

```rust
use pluto_lib::recipe_api_v1::{AssemblyOutcome, AssemblyRegistration, AssemblyRequest, GridFacts, RecipeFacts, RecipeForm, prepare_assembly, register_assembly};

register_assembly(AssemblyRegistration {
    id: "example:watcher",
    interest: |recipe| recipe.result_item == Some("minecraft:apple"),
    callback: |_| pluto_lib::recipe_api_v1::AssemblyDecision::Pass,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: None };
let evaluation = prepare_assembly(&facts).unwrap();
let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
assert_eq!(evaluation.evaluate(&request), AssemblyOutcome::Native);
```

### `AssemblyEvaluation::evaluate`

`fn evaluate(&self, request: &AssemblyRequest<'_>) -> AssemblyOutcome`. Runs every snapshotted callback: any denial wins, otherwise the first assembling provider in lexical registration order supplies the result's custom data, and if none does the outcome is `Native`.

```rust
use pluto_lib::recipe_api_v1::{AssemblyDecision, AssemblyOutcome, AssemblyRegistration, AssemblyRequest, GridFacts, RecipeFacts, RecipeForm, prepare_assembly, register_assembly};
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

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
let evaluation = prepare_assembly(&facts).unwrap();
let request = AssemblyRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
let AssemblyOutcome::Assembled { owner, custom_data } = evaluation.evaluate(&request) else { panic!("expected an assembly") };
assert_eq!(owner, "example:blessed");
assert_eq!(custom_data.get("blessed"), Some(&Nbt::Byte(1)));
```

### `register_assembly`

`fn register_assembly(entry: AssemblyRegistration) -> Result<(), RegistrationError>`. Registers one assembly provider once during startup; there is no unregister path.

```rust
use pluto_lib::recipe_api_v1::{AssemblyRegistration, prepare_assembly, RecipeFacts, RecipeForm, register_assembly};

register_assembly(AssemblyRegistration {
    id: "example:registered_provider",
    interest: |recipe| recipe.result_item == Some("minecraft:golden_apple"),
    callback: |_| pluto_lib::recipe_api_v1::AssemblyDecision::Pass,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
assert!(prepare_assembly(&facts).is_some());
```

### `has_assembly_providers`

`fn has_assembly_providers() -> bool`. Whether any assembly provider has been registered.

```rust
use pluto_lib::recipe_api_v1::{AssemblyRegistration, has_assembly_providers, register_assembly};

assert!(!has_assembly_providers());
register_assembly(AssemblyRegistration {
    id: "example:counted_provider",
    interest: |_| true,
    callback: |_| pluto_lib::recipe_api_v1::AssemblyDecision::Pass,
}).unwrap();
assert!(has_assembly_providers());
```

### `prepare_assembly`

`fn prepare_assembly(recipe: &RecipeFacts<'_>) -> Option<AssemblyEvaluation>`. Interest runs on cheap borrowed recipe facts; `None` — when no provider is interested — avoids all grid and result capture in the adapter, even with unrelated registrations.

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

let facts = RecipeFacts { form: RecipeForm::Shaped { width: 1, height: 1 }, result_item: Some("minecraft:golden_apple"), result_count: 1, id: None };
assert!(prepare_assembly(&facts).is_some());
let other = RecipeFacts { result_item: Some("minecraft:apple"), ..facts };
assert!(prepare_assembly(&other).is_none());
```

### `AdmissionRequest`

What an admission callback answers: `recipe` is the recipe's `RecipeFacts`, `grid` the cropped grid's `GridFacts`, and `result` the result template's native created stack, if any (not a proposed output).

```rust
use pluto_lib::recipe_api_v1::{AdmissionRequest, GridFacts, RecipeFacts, RecipeForm};

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("minecraft:apple") };
let request = AdmissionRequest {
    recipe: &facts,
    grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] },
    result: None,
};
assert_eq!(request.recipe.id, Some("minecraft:apple"));
assert_eq!(request.grid.width, 0);
assert!(request.result.is_none());
```

### `AdmissionDecision`

What one admission policy decides for one grid: `Pass` (the native result stands) or `Deny` (the recipe does not match this grid after all).

```rust
use pluto_lib::recipe_api_v1::AdmissionDecision;

assert_eq!(AdmissionDecision::Pass, AdmissionDecision::Pass);
assert_ne!(AdmissionDecision::Deny, AdmissionDecision::Pass);
```

### `AdmissionInterest`

`type AdmissionInterest = for<'a> fn(&RecipeFacts<'a>) -> bool`. The cheap interest check of an admission policy, decided on recipe facts before any grid or stack is captured.

```rust
use pluto_lib::recipe_api_v1::{AdmissionInterest, RecipeFacts, RecipeForm};

fn interested(recipe: &RecipeFacts<'_>) -> bool { recipe.id == Some("example:gated") }
let interest: AdmissionInterest = interested;
let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
assert!(interest(&facts));
```

### `AdmissionCallback`

`type AdmissionCallback = for<'a> fn(&AdmissionRequest<'a>) -> AdmissionDecision`. The callback an interested admission policy runs for one grid request.

```rust
use pluto_lib::recipe_api_v1::{AdmissionCallback, AdmissionDecision, AdmissionRequest, GridFacts, RecipeFacts, RecipeForm};

fn pass(_: &AdmissionRequest<'_>) -> AdmissionDecision { AdmissionDecision::Pass }
let callback: AdmissionCallback = pass;
let facts = RecipeFacts { form: RecipeForm::Special, result_item: None, result_count: 1, id: None };
let request = AdmissionRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
assert_eq!(callback(&request), AdmissionDecision::Pass);
```

### `AdmissionRegistration`

One admission policy to register: `id` is its registration id, `interest` its `AdmissionInterest` check, and `callback` its `AdmissionCallback`.

```rust
use pluto_lib::recipe_api_v1::{AdmissionRegistration, RecipeFacts, RecipeForm};

let registration = AdmissionRegistration {
    id: "example:config_gate",
    interest: |recipe| recipe.id == Some("example:gated"),
    callback: |_| pluto_lib::recipe_api_v1::AdmissionDecision::Pass,
};
assert_eq!(registration.id, "example:config_gate");
let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
assert!((registration.interest)(&facts));
```

### `AdmissionOutcome`

The result of evaluating the interested policies for one request: `Native` (no policy denied) or `Denied` naming the `first` denying policy.

```rust
use pluto_lib::recipe_api_v1::AdmissionOutcome;

assert_eq!(AdmissionOutcome::Native, AdmissionOutcome::Native);
assert_eq!(AdmissionOutcome::Denied { first: "example:veto" }, AdmissionOutcome::Denied { first: "example:veto" });
assert_ne!(AdmissionOutcome::Native, AdmissionOutcome::Denied { first: "example:veto" });
```

### `AdmissionEvaluation`

An ephemeral snapshot of the interested policies' callbacks, returned by `prepare_admission`; the adapter evaluates it synchronously with the same recipe facts. It is not a recipe or world identity or authority token.

```rust
use pluto_lib::recipe_api_v1::{AdmissionOutcome, AdmissionRegistration, AdmissionRequest, GridFacts, RecipeFacts, RecipeForm, prepare_admission, register_admission};

register_admission(AdmissionRegistration {
    id: "example:watcher",
    interest: |recipe| recipe.id == Some("example:watched"),
    callback: |_| pluto_lib::recipe_api_v1::AdmissionDecision::Pass,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:watched") };
let evaluation = prepare_admission(&facts).unwrap();
let request = AdmissionRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
assert_eq!(evaluation.evaluate(&request), AdmissionOutcome::Native);
```

### `AdmissionEvaluation::evaluate`

`fn evaluate(&self, request: &AdmissionRequest<'_>) -> AdmissionOutcome`. Runs every snapshotted callback: any denial wins, pass cannot grant matching or provide an output, and the first denying policy in lexical registration order is the one reported.

```rust
use pluto_lib::recipe_api_v1::{AdmissionDecision, AdmissionOutcome, AdmissionRegistration, AdmissionRequest, GridFacts, RecipeFacts, RecipeForm, prepare_admission, register_admission};

register_admission(AdmissionRegistration {
    id: "example:gate",
    interest: |recipe| recipe.id == Some("example:gated"),
    callback: |_| AdmissionDecision::Deny,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
let evaluation = prepare_admission(&facts).unwrap();
let request = AdmissionRequest { recipe: &facts, grid: GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] }, result: None };
assert_eq!(evaluation.evaluate(&request), AdmissionOutcome::Denied { first: "example:gate" });
```

### `register_admission`

`fn register_admission(entry: AdmissionRegistration) -> Result<(), RegistrationError>`. Registers one admission policy once during startup; both of its functions must be read-only and deterministic for their arguments, and there is no unregister path.

```rust
use pluto_lib::recipe_api_v1::{AdmissionRegistration, RecipeFacts, RecipeForm, prepare_admission, register_admission};

register_admission(AdmissionRegistration {
    id: "example:registered_policy",
    interest: |recipe| recipe.id == Some("example:gated"),
    callback: |_| pluto_lib::recipe_api_v1::AdmissionDecision::Pass,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
assert!(prepare_admission(&facts).is_some());
```

### `has_admission_providers`

`fn has_admission_providers() -> bool`. Whether any admission policy has been registered.

```rust
use pluto_lib::recipe_api_v1::{AdmissionRegistration, has_admission_providers, register_admission};

assert!(!has_admission_providers());
register_admission(AdmissionRegistration {
    id: "example:counted_policy",
    interest: |_| true,
    callback: |_| pluto_lib::recipe_api_v1::AdmissionDecision::Pass,
}).unwrap();
assert!(has_admission_providers());
```

### `prepare_admission`

`fn prepare_admission(recipe: &RecipeFacts<'_>) -> Option<AdmissionEvaluation>`. Interest runs on cheap borrowed recipe facts; `None` — when no policy is interested — avoids all grid and result capture in the adapter, even with unrelated registrations.

```rust
use pluto_lib::recipe_api_v1::{AdmissionRegistration, RecipeFacts, RecipeForm, prepare_admission, register_admission};

register_admission(AdmissionRegistration {
    id: "example:gate",
    interest: |recipe| recipe.id == Some("example:gated"),
    callback: |_| pluto_lib::recipe_api_v1::AdmissionDecision::Deny,
}).unwrap();

let facts = RecipeFacts { form: RecipeForm::Shapeless, result_item: Some("minecraft:apple"), result_count: 1, id: Some("example:gated") };
assert!(prepare_admission(&facts).is_some());
let other = RecipeFacts { id: Some("example:other"), ..facts };
assert!(prepare_admission(&other).is_none());
```

## Special recipes

A special recipe's result depends on the grid's concrete stacks — their materials, damage or saved data — so it has no fixed result template the engine could assemble, and the engine's own special-recipe list is closed. A mod registers the recipe itself: the provider is consulted only when the native lookup found no match for the grid, inspects the cropped grid, and either passes or supplies the complete result stack. The registration id is the recipe identity the lookup reports, in the `<namespace>:<path>` form the recipe file would declare. Taking the result consumes one item from every occupied grid cell, as for any crafting recipe, so a provider that answers a tool-and-ingredient pair produces exactly the original's craft. Providers must be read-only and deterministic: the lookup re-evaluates grids on every inventory change.

```rust
use pluto_lib::recipe_api_v1::{GridFacts, SpecialRecipeRegistration, SpecialResult, StackView, evaluate_special_recipe, register_special_recipe};

register_special_recipe(SpecialRecipeRegistration {
    id: "example:apple_pie",
    callback: |grid| {
        let apples = grid.cells.iter().flatten().filter(|cell| cell.id == "minecraft:apple").count();
        (apples == 2).then(|| SpecialResult { item: "example:pie".to_owned(), count: 1, damage: 0, custom_data: None, remainders: None })
    },
}).unwrap();

let cells = [Some(StackView { id: "minecraft:apple", count: 1, damage: 0, custom_data: None }); 2];
let grid = GridFacts { width: 2, height: 1, left: 0, top: 0, cells: &cells };
let (name, result) = evaluate_special_recipe(&grid).unwrap();
assert_eq!(name, "example:apple_pie");
assert_eq!(result.item, "example:pie");
```

### `SpecialResult`

The complete result stack a special recipe produces for one grid: `item` is the result's item id, `count` how many, `damage` its damage, and `custom_data` its `minecraft:custom_data` compound when the result carries one (a repaired tool's saved tool data, for instance). `remainders` names the cells of the evaluated (cropped) grid this craft does not spend — an entry is the stack that stays in that cell, the shape of a recipe whose `getRemainingItems` leaves an unneeded ingredient in place once the craft's need is covered; the field or an entry being `None` spends the cell, and the engine's own remainder for it stands. The crafting lookup applies the stays over the engine's remainders; the game's own take path does not consult providers, so a special crafted there still spends every occupied cell.

```rust
use pluto_lib::recipe_api_v1::SpecialResult;

let result = SpecialResult { item: "example:pie".to_owned(), count: 1, damage: 0, custom_data: None, remainders: None };
assert_eq!(result.item, "example:pie");
assert_eq!((result.count, result.damage), (1, 0));
assert!(result.custom_data.is_none());
assert!(result.remainders.is_none());
```

### `SpecialCallback`

`type SpecialCallback = for<'a> fn(&GridFacts<'a>) -> Option<SpecialResult>`. Answers one cropped grid: a result when the recipe matches it, or None to pass.

```rust
use pluto_lib::recipe_api_v1::{GridFacts, SpecialCallback, SpecialResult};

fn never(_: &GridFacts<'_>) -> Option<SpecialResult> { None }
let callback: SpecialCallback = never;
let grid = GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] };
assert!(callback(&grid).is_none());
```

### `SpecialRecipeRegistration`

One special recipe: `id` is the recipe identity reported for a match, in `<namespace>:<path>` form (the id the recipe file would declare), and `callback` answers grids for it.

```rust
use pluto_lib::recipe_api_v1::{SpecialRecipeRegistration, register_special_recipe};

register_special_recipe(SpecialRecipeRegistration {
    id: "example:apple_pie",
    callback: |_| None,
}).unwrap();
```

### `register_special_recipe`

`fn register_special_recipe(entry: SpecialRecipeRegistration) -> Result<(), RegistrationError>`. Registers one special recipe once during startup; there is no unregister path.

```rust
use pluto_lib::recipe_api_v1::{SpecialRecipeRegistration, has_special_recipes, register_special_recipe};

assert!(!has_special_recipes());
register_special_recipe(SpecialRecipeRegistration { id: "example:apple_pie", callback: |_| None }).unwrap();
assert!(has_special_recipes());
```

### `has_special_recipes`

`fn has_special_recipes() -> bool`. Whether any special recipe is registered, so the lookup can skip the provider evaluation entirely otherwise.

```rust
use pluto_lib::recipe_api_v1::{SpecialRecipeRegistration, has_special_recipes, register_special_recipe};

register_special_recipe(SpecialRecipeRegistration { id: "example:apple_pie", callback: |_| None }).unwrap();
assert!(has_special_recipes());
```

### `evaluate_special_recipe`

`fn evaluate_special_recipe(grid: &GridFacts<'_>) -> Option<(&'static str, SpecialResult)>`. The first answering provider's recipe identity and result, in lexical registration order, or None when no provider answers the grid.

```rust
use pluto_lib::recipe_api_v1::{GridFacts, SpecialRecipeRegistration, SpecialResult, evaluate_special_recipe, register_special_recipe};

register_special_recipe(SpecialRecipeRegistration {
    id: "example:apple_pie",
    callback: |_| Some(SpecialResult { item: "example:pie".to_owned(), count: 1, damage: 0, custom_data: None, remainders: None }),
}).unwrap();
let grid = GridFacts { width: 0, height: 0, left: 0, top: 0, cells: &[] };
let (name, result) = evaluate_special_recipe(&grid).unwrap();
assert_eq!(name, "example:apple_pie");
assert_eq!(result.count, 1);
```

### `special_recipe_ids`

`fn special_recipe_ids() -> Vec<&'static str>`. The ids of every registered special recipe, in lexical order. The game glue mirrors each id into the native inventory registry so a provider can be resolved under its own recipe name.

```rust
use pluto_lib::recipe_api_v1::{SpecialRecipeRegistration, register_special_recipe, special_recipe_ids};

register_special_recipe(SpecialRecipeRegistration { id: "example:apple_pie", callback: |_| None }).unwrap();
assert_eq!(special_recipe_ids(), vec!["example:apple_pie"]);
```

### `SpecialRecipeSink`

`type SpecialRecipeSink = fn(&'static str)`. The sink notified with the id of each special recipe as it registers: the game glue's mirror into the native inventory registry.

```rust
use pluto_lib::recipe_api_v1::SpecialRecipeSink;

fn record(_id: &'static str) {}
let sink: SpecialRecipeSink = record;
sink("example:apple_pie");
```

### `set_special_recipe_sink`

`fn set_special_recipe_sink(sink: SpecialRecipeSink)`. Install the special-recipe registration sink, once during startup; there is no uninstall. Registrations from before the sink was installed are covered by `special_recipe_ids`.

```rust
use pluto_lib::recipe_api_v1::{SpecialRecipeRegistration, register_special_recipe, set_special_recipe_sink};

fn record(_id: &'static str) {}
set_special_recipe_sink(record);
register_special_recipe(SpecialRecipeRegistration { id: "example:apple_pie", callback: |_| None }).unwrap();
```

## Cooking lookups

### `CookingRecipe`

One cooking recipe of the game's own book (the furnace family), as a lookup answers it: `result` the result's item id, `result_count` the result's count for one input item, `experience` the experience the recipe grants per item cooked and `time` the cooking time in ticks.

```rust
use pluto_lib::recipe_api_v1::CookingRecipe;

let recipe = CookingRecipe { result: "minecraft:cooked_beef".into(), result_count: 1, experience: 0.35, time: 200 };
assert_eq!(recipe.result, "minecraft:cooked_beef");
assert_eq!((recipe.result_count, recipe.time), (1, 200));
```

### `CookingLookup`

`type CookingLookup = fn(&str, &str) -> Option<CookingRecipe>`. The adapter's cooking lookup: the first recipe of a kind (a recipe type id such as `minecraft:smelting`) that takes an item, or None when the book has none for it.

```rust
use pluto_lib::recipe_api_v1::{CookingLookup, CookingRecipe};

let lookup: CookingLookup = |kind, item| {
    if kind == "minecraft:smelting" && item == "minecraft:beef" {
        Some(CookingRecipe { result: "minecraft:cooked_beef".into(), result_count: 1, experience: 0.35, time: 200 })
    } else {
        None
    }
};
assert_eq!(lookup("minecraft:smelting", "minecraft:beef").map(|recipe| recipe.result), Some("minecraft:cooked_beef".into()));
assert!(lookup("minecraft:smelting", "minecraft:stone").is_none());
```

### `set_cooking_lookup`

`fn set_cooking_lookup(lookup: CookingLookup)`. Installs the adapter's cooking lookup, once during startup. There is no uninstall.

```rust
use pluto_lib::recipe_api_v1::{CookingRecipe, cooking_recipe, set_cooking_lookup};

set_cooking_lookup(|kind, item| {
    if kind == "minecraft:smelting" && item == "minecraft:beef" {
        Some(CookingRecipe { result: "minecraft:cooked_beef".into(), result_count: 1, experience: 0.35, time: 200 })
    } else {
        None
    }
});
assert_eq!(cooking_recipe("minecraft:smelting", "minecraft:beef").map(|recipe| recipe.result), Some("minecraft:cooked_beef".into()));
```

### `cooking_recipe`

`fn cooking_recipe(kind: &str, item: &str) -> Option<CookingRecipe>`. The first cooking recipe of `kind` that takes `item`, as the game's recipe book answers it (its ingredient naming the item; a stack-sensitive ingredient test is the engine's and is not replayed). None when the book has none, or when no adapter is installed.

```rust
use pluto_lib::recipe_api_v1::{CookingRecipe, cooking_recipe, set_cooking_lookup};

assert!(cooking_recipe("minecraft:smelting", "minecraft:beef").is_none(), "no adapter installed yet");
set_cooking_lookup(|kind, item| {
    if kind == "minecraft:smelting" && item == "minecraft:beef" {
        Some(CookingRecipe { result: "minecraft:cooked_beef".into(), result_count: 1, experience: 0.35, time: 200 })
    } else {
        None
    }
});
let recipe = cooking_recipe("minecraft:smelting", "minecraft:beef").unwrap();
assert_eq!((recipe.result.as_str(), recipe.result_count, recipe.time), ("minecraft:cooked_beef", 1, 200));
assert!(cooking_recipe("minecraft:blasting", "minecraft:beef").is_none());
```

## Crafting lookups

### `CraftingMatch`

One match of the game's crafting book for a grid, as a lookup answers it: `recipe` the matched recipe's id, qualified (`minecraft:oak_planks` for a bare table name, the whole id for anyone else's recipe), `result` the assembled result stack, and `remainders` what each grid cell holds after one craft, aligned with the queried grid (`size * size` entries, row-major) — a spent cell's crafting remainder (a milk bucket's empty bucket), or None where the cell empties. Taking the result spends one item from every occupied cell and leaves these stacks in their cells; a remainder that cannot stay in a shrunken cell is the taker's to hand to the player, as in the game's own take logic.

```rust
use pluto_lib::recipe_api_v1::CraftingMatch;
use pluto_lib::{Identifier, ItemSlot};

let matched = CraftingMatch {
    recipe: "minecraft:crafting_table".to_owned(),
    result: ItemSlot::new(Identifier::parse("minecraft:crafting_table").unwrap(), 1),
    remainders: vec![None; 9],
};
assert_eq!(matched.recipe, "minecraft:crafting_table");
assert_eq!(matched.result.count, 1);
assert_eq!(matched.remainders.len(), 9);
```

### `CraftingLookup`

`type CraftingLookup = fn(&[Option<ItemSlot>], usize) -> Option<CraftingMatch>`. The adapter's crafting lookup: the crafting book's match for a grid of `size` rows and columns (the cells row-major, `size * size` of them), or None when the book matches nothing.

```rust
use pluto_lib::recipe_api_v1::{CraftingLookup, CraftingMatch};
use pluto_lib::{Identifier, ItemSlot};

let lookup: CraftingLookup = |grid, size| {
    let planks = grid.iter().flatten().filter(|stack| stack.item.to_string() == "minecraft:oak_planks").count();
    (size == 3 && planks == 4).then(|| CraftingMatch {
        recipe: "minecraft:crafting_table".to_owned(),
        result: ItemSlot::new(Identifier::parse("minecraft:crafting_table").unwrap(), 1),
        remainders: vec![None; size * size],
    })
};
let mut grid = vec![None; 9];
for index in [0, 1, 3, 4] {
    grid[index] = Some(ItemSlot::new(Identifier::parse("minecraft:oak_planks").unwrap(), 1));
}
assert_eq!(lookup(&grid, 3).map(|matched| matched.recipe), Some("minecraft:crafting_table".to_owned()));
assert!(lookup(&grid, 2).is_none());
```

### `set_crafting_lookup`

`fn set_crafting_lookup(lookup: CraftingLookup)`. Installs the adapter's crafting lookup, once during startup. There is no uninstall.

```rust
use pluto_lib::recipe_api_v1::{CraftingMatch, crafting_match, set_crafting_lookup};
use pluto_lib::{Identifier, ItemSlot};

set_crafting_lookup(|grid, size| {
    let planks = grid.iter().flatten().filter(|stack| stack.item.to_string() == "minecraft:oak_planks").count();
    (size == 3 && planks == 4).then(|| CraftingMatch {
        recipe: "minecraft:crafting_table".to_owned(),
        result: ItemSlot::new(Identifier::parse("minecraft:crafting_table").unwrap(), 1),
        remainders: vec![None; size * size],
    })
});
let mut grid = vec![None; 9];
for index in [0, 1, 3, 4] {
    grid[index] = Some(ItemSlot::new(Identifier::parse("minecraft:oak_planks").unwrap(), 1));
}
assert_eq!(crafting_match(&grid, 3).map(|matched| matched.recipe), Some("minecraft:crafting_table".to_owned()));
```

### `crafting_match`

`fn crafting_match(grid: &[Option<ItemSlot>], size: usize) -> Option<CraftingMatch>`. The crafting book's match for `grid`, as the game's own crafting answers it: the first recipe in the book's order whose match the grid satisfies — admission policies, assembly providers and special markers all apply, as they do at the game's own table — assembled for this grid. A matched special recipe whose provider declares remainders leaves those cells' stays in place of the engine's remainders. None when the book matches nothing, or when no adapter is installed.

```rust
use pluto_lib::recipe_api_v1::{CraftingMatch, crafting_match, set_crafting_lookup};
use pluto_lib::{Identifier, ItemSlot};

assert!(crafting_match(&vec![None; 9], 3).is_none(), "no adapter installed yet");
set_crafting_lookup(|grid, size| {
    let planks = grid.iter().flatten().filter(|stack| stack.item.to_string() == "minecraft:oak_planks").count();
    (size == 3 && planks == 4).then(|| CraftingMatch {
        recipe: "minecraft:crafting_table".to_owned(),
        result: ItemSlot::new(Identifier::parse("minecraft:crafting_table").unwrap(), 1),
        remainders: vec![None; size * size],
    })
});
let mut grid = vec![None; 9];
for index in [0, 1, 3, 4] {
    grid[index] = Some(ItemSlot::new(Identifier::parse("minecraft:oak_planks").unwrap(), 1));
}
let matched = crafting_match(&grid, 3).unwrap();
assert_eq!(matched.result.item.to_string(), "minecraft:crafting_table");
grid[0] = None;
assert!(crafting_match(&grid, 3).is_none(), "three planks match nothing");
```
