// Lookup uses one world's resolved tags and ordered datapack recipes. A fluid
// tag is accepted only when it identifies one still fluid; output tags are
// accepted only when their choice is unambiguous. This avoids guessing a
// missing mod's fluid or a namespace preference.
use std::collections::{BTreeMap, HashMap, HashSet};
use serde_json::Value;
use super::tconstruct_source_melting_recipes::{AlloyRecipe, BasinFillingRecipe, BucketFillingRecipe, CastingRecipe, CompositeCastingRecipe, CompositeFluidRecipe, DuplicationRecipe, Ingredient, FluidFuel, FluidSpec, FoundryRecipe, MaterialCastingRecipe, MaterialFluidRecipe, MeltingRecipe, MoldingRecipe, Output};

/// Supported source material predicates. Unknown forms fail closed, including
/// inside negation, rather than becoming unrestricted.
pub(super) fn material_predicate(value: Option<&Value>, material: &str,
    tagged: &impl Fn(&str, &str) -> bool, supports: &impl Fn(&str, &str) -> Option<bool>, extra: &impl Fn(&str, &Value, &str) -> Option<bool>) -> Option<bool> {
    fn test(v: &Value, material: &str, tagged: &impl Fn(&str, &str) -> bool,
        supports: &impl Fn(&str, &str) -> Option<bool>, extra: &impl Fn(&str, &Value, &str) -> Option<bool>) -> Option<bool> {
        if v.is_null() { return Some(true); }
        let compact;
        let obj = if let Some(id) = v.as_str() {
            compact = serde_json::json!({"type": id}); compact.as_object()?
        } else { v.as_object()? };
        let id = obj.get("type")?.as_str()?;
        let base = material.split_once('#').map_or(material, |(base, _)| base);
        let values = |name: &str| -> Option<Vec<&str>> {
            let v = obj.get(name)?;
            if let Some(s) = v.as_str() { Some(vec![s]) }
            else { v.as_array()?.iter().map(Value::as_str).collect() }
        };
        match id {
            "mantle:any" => Some(true), "mantle:none" => Some(false),
            "mantle:tag" => Some(tagged(obj.get("tag")?.as_str()?, base)),
            "tconstruct:id" => Some(values("id")?.contains(&base)),
            "tconstruct:variant" => Some(values("variant")?.contains(&material)),
            "tconstruct:has_part" => supports(material, obj.get("part")?.as_str()?),
            "mantle:and" | "mantle:or" => {
                let children = obj.get("predicates")?.as_array()?;
                if children.len() < 2 { return None; }
                let results: Vec<bool> = children.iter().map(|v| test(v, material, tagged, supports, extra)).collect::<Option<_>>()?;
                Some(if id == "mantle:and" { results.iter().all(|v| *v) } else { results.iter().any(|v| *v) })
            },
            "mantle:inverted" => {
                let mut inner = obj.clone();
                let inner_type = inner.remove("inverted_type")?.as_str()?.to_owned();
                inner.insert("type".into(), Value::String(inner_type));
                Some(!test(&Value::Object(inner), material, tagged, supports, extra)?)
            },
            _ => extra(id, v, material),
        }
    }
    match value { None | Some(Value::Null) => Some(true), Some(v) => test(v, material, tagged, supports, extra) }
}

pub(super) struct Catalog<'a> {
    pub recipes: &'a BTreeMap<String, Vec<u8>>,
    pub item_tags: &'a HashMap<String, HashSet<String>>,
    pub fluid_tags: &'a HashMap<String, HashSet<String>>,
    pub known_items: &'a HashSet<String>,
    pub known_fluids: &'a HashSet<String>,
}
impl Catalog<'_> {
    /// Route a matching fixed source recipe exclusively through its guarded path.
    pub(super) fn has_slimeskull_swap(&self, basin: bool, input: &str, conditions: &impl Fn(&[Value]) -> bool) -> bool {
        basin && input == "tconstruct:slime_helmet" && self.recipes.values().any(|b|
            super::tconstruct_source_melting_recipes::slimeskull_part_swap(b, conditions))
    }
    pub(super) fn slimeskull_part_swap(&self, basin: bool, input: &str, old: &str, fluid: &str,
        supports: &impl Fn(&str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String)> {
        if !self.has_slimeskull_swap(basin, input, conditions) || !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|v| v.contains(id));
        let mut matching = self.recipes.iter().filter_map(|(id, b)| {
            let r = super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(b, conditions)?;
            (r.basin && r.tools.matches(input, &in_tag)).then_some((id, b))
        });
        let (id, bytes) = matching.next()?;
        if matching.next().is_some() || !super::tconstruct_source_melting_recipes::slimeskull_part_swap(bytes, conditions) { return None; }
        let mut found = None;
        for raw in self.recipes.values() {
            let Some(material) = MaterialFluidRecipe::parse(raw, conditions) else { continue; };
            let canonical = |id: &str| id.strip_suffix("#default").unwrap_or(id).to_owned();
            if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material)
                || canonical(old) == canonical(&material.material) { continue; }
            let amount = material.fluid.amount.checked_mul(5)?;
            let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * ((amount as f32 / 90.0_f32).sqrt() as f64)).round();
            if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
            let mut spec = material.fluid.clone(); spec.amount = amount;
            let recipe = CastingRecipe { basin: true, retextured: false, cast: Some(Ingredient::Item(input.into())), consumes_cast: true,
                switch_slots: false, fluid: spec, output: Output::Item(input.into()), cooling_time: time as u32 };
            let value = (id.clone(), recipe, material.material);
            if found.as_ref().is_some_and(|prior| prior != &value) { return None; }
            found = Some(value);
        }
        found
    }
    /// A supported tool-cast and explicit part-swap on the same workstation
    /// must not be resolved by dispatch order, even if one later fails validation.
    pub(super) fn tool_swap_overlap(&self, basin: bool, input: &str, conditions: &impl Fn(&[Value]) -> bool) -> bool {
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let tool = self.recipes.values().any(|b| {
            super::tconstruct_source_melting_recipes::FirstMaterialCastingRecipe::parse(b, conditions)
                .is_some_and(|r| r.basin == basin && r.result == input)
            || super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(b, conditions)
                .is_some_and(|r| basin && r.result == input)
        });
        let part = self.recipes.values().filter_map(|b| super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(b, conditions))
            .any(|r| r.basin == basin && r.tools.matches(input, &in_tag));
        tool && part
    }
    /// The default fluid-swapping form of the five supported tool casts.
    /// Composite fluids and unrecognized tool-casting forms are not inferred.
    pub(super) fn tool_fluid_swap(&self, basin: bool, input: &str, old: &str, fluid: &str,
        supports: &impl Fn(&str, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        if self.tool_swap_overlap(basin, input, conditions) { return None; }
        let mut found = None;
        for (id, bytes) in self.recipes {
            let first = super::tconstruct_source_melting_recipes::FirstMaterialCastingRecipe::parse(bytes, conditions)
                .filter(|r| r.basin == basin && r.result == input)
                .map(|r| (r.item_cost, r.fluid_stat));
            let wings = super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(bytes, conditions)
                .filter(|r| basin && r.result == input).map(|r| (r.item_cost, "tconstruct:slime"));
            let Some((cost, stat)) = first.or(wings) else { continue; };
            for raw in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(raw, conditions) else { continue; };
                let canonical = |id: &str| id.strip_suffix("#default").unwrap_or(id).to_owned();
                if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material, stat)
                    || canonical(old) == canonical(&material.material) { continue; }
                let amount = material.fluid.amount.checked_mul(cost)?;
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * ((amount as f32 / 90.0_f32).sqrt() as f64)).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: Some(Ingredient::Item(input.into())), consumes_cast: true,
                    switch_slots: false, fluid: spec, output: Output::Item(input.into()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, material.material);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    /// Match only the four source first-material recipes and resolved material fluids.
    pub(super) fn first_material_casting(&self, basin: bool, input: &str, first: &str, fluid: &str,
        supports: &impl Fn(&str, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String, String)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        if self.recipes.values().filter_map(|b| super::tconstruct_source_melting_recipes::SlimeskullCastingRecipe::parse(b, conditions))
            .any(|r| basin && r.cast.matches(input, &in_tag)) || self.recipes.values().filter_map(|b| super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(b, conditions))
            .any(|r| basin && r.cast.matches(input, &in_tag)) { return None; }
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(tool) = super::tconstruct_source_melting_recipes::FirstMaterialCastingRecipe::parse(bytes, conditions) else { continue; };
            if tool.basin != basin || !tool.cast.matches(input, &in_tag) || !self.known_items.contains(&tool.result) || !supports(first, tool.first_stat) { continue; }
            for raw in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(raw, conditions) else { continue; };
                if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material, tool.fluid_stat) { continue; }
                let amount = material.fluid.amount.checked_mul(tool.item_cost)?;
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * ((amount as f32 / 90.0_f32).sqrt() as f64)).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: Some(tool.cast.clone()), consumes_cast: true,
                    switch_slots: false, fluid: spec, output: Output::Item(tool.result.clone()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, tool.result.clone(), material.material);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    /// Resolve the source's consumed-offset head plus slime material form.
    pub(super) fn slimeskull_casting(&self, basin: bool, input: &str, fluid: &str,
        supports: &impl Fn(&str, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String, String)> {
        if !basin || !self.known_items.contains(input) || !self.known_items.contains("tconstruct:slime_helmet") || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        if self.recipes.values().filter_map(|bytes| super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(bytes, conditions))
            .any(|recipe| recipe.cast.matches(input, &in_tag)) { return None; }
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(tool) = super::tconstruct_source_melting_recipes::SlimeskullCastingRecipe::parse(bytes, conditions) else { continue; };
            if !tool.cast.matches(input, &in_tag) || !supports(&tool.material, "tconstruct:skull") { continue; }
            for raw in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(raw, conditions) else { continue; };
                if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material, "tconstruct:slime") { continue; }
                let amount = material.fluid.amount.checked_mul(tool.item_cost)?;
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin: true, retextured: false, cast: Some(tool.cast.clone()),
                    consumes_cast: true, switch_slots: false, fluid: spec,
                    output: Output::Item("tconstruct:slime_helmet".into()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, tool.material.clone(), material.material);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    /// Resolve the consumed-cast slimelytra recipe against one material fluid.
    pub(super) fn slimelytra_casting(&self, basin: bool, input: &str, fluid: &str,
        supports: &impl Fn(&str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String)> {
        if !basin || !self.known_items.contains(input) || !self.known_items.contains("tconstruct:slime_wings") || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        // Avoid consuming an item if an overlapping supported tool-casting
        // form could also match it after a datapack change.
        if self.recipes.values().filter_map(|bytes| super::tconstruct_source_melting_recipes::SlimeskullCastingRecipe::parse(bytes, conditions))
            .any(|recipe| recipe.cast.matches(input, &in_tag)) { return None; }
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(tool) = super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(bytes, conditions) else { continue; };
            if !tool.cast.matches(input, &in_tag) { continue; }
            for raw in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(raw, conditions) else { continue; };
                if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material) { continue; }
                let amount = material.fluid.amount.checked_mul(tool.item_cost)?;
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin: true, retextured: false, cast: Some(tool.cast.clone()),
                    consumes_cast: true, switch_slots: false, fluid: spec,
                    output: Output::Item(tool.result.clone()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, material.material);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    pub(super) fn potion_casting(&self, basin: bool, input: &str, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String)> {
        if !self.known_items.contains(input) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, CastingRecipe, String)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = super::tconstruct_source_melting_recipes::potion_casting(bytes, conditions) else { continue; };
            if recipe.basin != basin || !self.matches_fluid(&recipe.fluid, fluid)
                || !recipe.cast.as_ref().is_some_and(|cast| cast.matches(input, &in_tag)) { continue; }
            let Some(output) = self.output(&recipe.output) else { continue; };
            if let Some((_, previous, previous_output)) = &found {
                if previous != &recipe || previous_output != &output { return None; }
            } else { found = Some((id.clone(), recipe, output)); }
        }
        found
    }
    pub(super) fn tip_clearing(&self, basin: bool, input: &str, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, CastingRecipe)> = None;
        for (id, bytes) in self.recipes {
            let Some(source) = super::tconstruct_source_melting_recipes::TipClearingRecipe::parse(bytes, conditions) else { continue; };
            // Only the source's simple item/tag forms are understood here.
            // A missing tag inside a difference cannot safely mean false.
            if !matches!(&source.tools, Ingredient::Item(_) | Ingredient::Tag(_)) { continue; }
            if source.basin != basin || !source.tools.matches(input, &in_tag) || !self.matches_fluid(&source.fluid, fluid) { continue; }
            let recipe = CastingRecipe { basin, retextured: false, cast: Some(source.tools), consumes_cast: true,
                switch_slots: false, fluid: source.fluid, output: Output::Item(input.to_owned()), cooling_time: source.cooling_time };
            if found.as_ref().is_some_and(|(_, old)| old != &recipe) { return None; }
            if found.is_none() { found = Some((id.clone(), recipe)); }
        }
        found
    }
    pub(super) fn composite_part_swap(&self, basin: bool, input: &str, fluid: &str, identities: &[String],
        supports: &impl Fn(usize, &str) -> bool, supports_ordinary: &impl Fn(usize, &str) -> bool,
        allowed: &impl Fn(Option<&Value>, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String, usize)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        // Normal casting has source precedence, even if our conservative
        // normal matcher rejected it as ambiguous or a later check failed.
        if self.recipes.values().filter_map(|bytes| super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(bytes, conditions))
            .filter(|swap| swap.basin == basin && swap.tools.matches(input, &in_tag))
            .any(|swap| self.recipes.values().filter_map(|bytes| MaterialFluidRecipe::parse(bytes, conditions))
                .any(|material| self.matches_fluid(&material.fluid, fluid) && allowed(swap.allowed_materials.as_ref(), &material.material) && supports_ordinary(swap.index, &material.material))) { return None; }
        let split = |id: &str| { let (base, variant) = id.split_once('#').unwrap_or((id, "")); (base.to_owned(), variant.to_owned()) };
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(swap) = super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(bytes, conditions) else { continue; };
            if swap.basin != basin || !swap.tools.matches(input, &in_tag) { continue; }
            let Some(current) = identities.get(swap.index) else { continue; };
            let (current_base, current_variant) = split(current);
            for fluid_bytes in self.recipes.values() {
                let Some(material) = CompositeFluidRecipe::parse(fluid_bytes, conditions) else { continue; };
                let (wanted_base, wanted_variant) = split(&material.input);
                if wanted_base != current_base || !(wanted_variant.is_empty() || wanted_variant == current_variant || wanted_variant == "default" && current_variant.is_empty())
                    || !self.matches_fluid(&material.fluid, fluid) || !allowed(swap.allowed_materials.as_ref(), &material.material) || !supports(swap.index, &material.material) { continue; }
                let Some(amount) = material.fluid.amount.checked_mul(swap.item_cost) else { continue; };
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: Some(swap.tools.clone()),
                    consumes_cast: true, switch_slots: false, fluid: spec,
                    output: Output::Item(input.to_owned()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, material.material, swap.index);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    pub(super) fn part_swap_casting(&self, basin: bool, input: &str, fluid: &str,
        supports: &impl Fn(usize, &str) -> bool, allowed: &impl Fn(Option<&Value>, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, CastingRecipe, String, usize)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(swap) = super::tconstruct_source_melting_recipes::PartSwapCastingRecipe::parse(bytes, conditions) else { continue; };
            if swap.basin != basin || !swap.tools.matches(input, &in_tag) { continue; }
            for fluid_bytes in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(fluid_bytes, conditions) else { continue; };
                if !self.matches_fluid(&material.fluid, fluid) || !allowed(swap.allowed_materials.as_ref(), &material.material) || !supports(swap.index, &material.material) { continue; }
                let Some(amount) = material.fluid.amount.checked_mul(swap.item_cost) else { continue; };
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: Some(swap.tools.clone()),
                    consumes_cast: true, switch_slots: false, fluid: spec,
                    output: Output::Item(input.to_owned()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, material.material, swap.index);
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
    fn matches_fluid(&self, spec: &FluidSpec, fluid: &str) -> bool {
        if !self.known_fluids.contains(fluid) { return false; }
        if spec.tagged { self.fluid_tags.get(&spec.id).is_some_and(|members| members.contains(fluid)) }
        else { spec.id == fluid }
    }
    fn resolved_fluid(&self, spec: &FluidSpec) -> Option<String> {
        if !spec.tagged { return self.known_fluids.contains(&spec.id).then(|| spec.id.clone()); }
        let members = self.fluid_tags.get(&spec.id)?;
        let mut still = members.iter().filter(|id| self.known_fluids.contains(*id)
            && !id.split_once(':').is_some_and(|(_, path)| path.starts_with("flowing_")));
        let id = still.next()?.clone();
        still.next().is_none().then_some(id)
    }
    fn output(&self, result: &Output) -> Option<String> {
        match result {
            Output::Item(id) | Output::WaterPotion(id) => self.known_items.contains(id).then(|| id.clone()),
            Output::Tag(tag) => {
                let mut values = self.item_tags.get(tag)?.iter().filter(|id| self.known_items.contains(*id));
                let value = values.next()?.clone();
                values.next().is_none().then_some(value)
            }
        }
    }
    pub(super) fn molding(&self, basin: bool, material: &str, pattern: Option<&str>, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, MoldingRecipe)> {
        if !self.known_items.contains(material) || pattern.is_some_and(|item| !self.known_items.contains(item)) { return None; }
        let in_tag = |tag: &str, item: &str| self.item_tags.get(tag).is_some_and(|items| items.contains(item));
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = MoldingRecipe::parse(bytes, conditions) else { continue; };
            if recipe.basin != basin || !self.known_items.contains(&recipe.result) || !recipe.material.matches(material, &in_tag) { continue; }
            if !match (&recipe.pattern, pattern) { (None, None) => true, (Some(ingredient), Some(item)) => ingredient.matches(item, &in_tag), _ => false } { continue; }
            if found.as_ref().is_some_and(|(_, previous)| previous != &recipe) { return None; }
            if found.is_none() { found = Some((id.clone(), recipe)); }
        }
        found
    }
    pub(super) fn fluid_fuel(&self, fluid: &str, available: u32, conditions: &impl Fn(&[Value]) -> bool) -> Option<FluidFuel> {
        for bytes in self.recipes.values() {
            let Some(fuel) = FluidFuel::parse(bytes, conditions) else { continue; };
            if available >= fuel.fluid.amount && self.matches_fluid(&fuel.fluid, fluid) { return Some(fuel); }
        }
        None
    }
    /// Match source ingredients in order, using each stored fluid at most once.
    /// Conditions and tags are evaluated against this world's snapshots.
    pub(super) fn alloy(&self, fluids: &[(String, u32)], capacity: u32, temperature: u32, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, Vec<(String, u32)>, String, u32)> {
        self.alloy_except(fluids, &[], capacity, temperature, conditions)
    }
    /// Stop before a recipe whose ingredients could select a tagged entry;
    /// selecting a later recipe could change source recipe ordering. The
    /// caller still includes tagged entries in capacity accounting.
    pub(super) fn alloy_except(&self, fluids: &[(String, u32)], tagged: &[String], capacity: u32, temperature: u32, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, Vec<(String, u32)>, String, u32)> {
        let total = fluids.iter().try_fold(0u32, |sum, (_, amount)| sum.checked_add(*amount))?;
        for (id, bytes) in self.recipes {
            let Some(recipe) = AlloyRecipe::parse(bytes, conditions) else { continue; };
            if recipe.temperature > temperature { continue; }
            if recipe.inputs.iter().any(|ingredient| tagged.iter().any(|fluid| self.matches_fluid(ingredient, fluid))) { return None; }
            let Some(output) = self.resolved_fluid(&recipe.output) else { continue; };
            let mut used = HashSet::new();
            let mut inputs = Vec::with_capacity(recipe.inputs.len());
            for (ingredient, catalyst) in recipe.inputs.iter().zip(&recipe.catalysts) {
                let matched = fluids.iter().enumerate().find(|(index, (fluid, amount))|
                    !used.contains(index) && *amount >= ingredient.amount && self.matches_fluid(ingredient, fluid));
                let Some((index, (fluid, _))) = matched else { break; };
                used.insert(index);
                if !*catalyst { inputs.push((fluid.clone(), ingredient.amount)); }
            }
            if used.len() == recipe.inputs.len() {
                let removed = inputs.iter().try_fold(0u32, |sum, (_, amount)| sum.checked_add(*amount))?;
                if recipe.output.amount.saturating_sub(removed) > capacity.saturating_sub(total) { continue; }
                return Some((id.clone(), inputs, output, recipe.output.amount));
            }
        }
        None
    }
    pub(super) fn melting(&self, item: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, MeltingRecipe, String)> {
        self.melting_with_container(item, None, conditions)
    }
    pub(super) fn melting_with_container(&self, item: &str, no_container: Option<bool>, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, MeltingRecipe, String)> {
        self.melting_with_rates(item, no_container, conditions, (Some(12), Some(8))).ok().flatten()
    }
    pub(super) fn melting_with_rates(&self, item: &str, no_container: Option<bool>, conditions: &impl Fn(&[Value]) -> bool, rates: (Option<u32>, Option<u32>)) -> Result<Option<(String, MeltingRecipe, String)>, ()> {
        if !self.known_items.contains(item) { return Ok(None); }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, MeltingRecipe, String)> = None;
        for (id, bytes) in self.recipes {
            let Some(input) = MeltingRecipe::candidate_input(bytes, conditions) else { continue; };
            if !input.matches_with_container(item, &in_tag, no_container).ok_or(())? { continue; }
            // Ignore malformed recipes as before. Neutral rates validate the
            // structure without rejecting representable configured outputs.
            if MeltingRecipe::parse_with_rates(bytes, conditions, (Some(9), Some(4))).is_none() { continue; }
            // A matched recipe with an unavailable rate must not fall through
            // to a different recipe or silently use a default.
            let recipe = MeltingRecipe::parse_with_rates(bytes, conditions, rates).ok_or(())?;
            let Some(fluid) = self.resolved_fluid(&recipe.output) else { continue; };
            if let Some((_, previous, previous_fluid)) = &found {
                if previous.temperature != recipe.temperature || previous.time != recipe.time
                    || previous.output.amount != recipe.output.amount || previous_fluid != &fluid { return Err(()); }
            } else { found = Some((id.clone(), recipe, fluid)); }
        }
        Ok(found)
    }
    pub(super) fn material_melting(&self, item: &str, material: &str, cost: u32, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        if !self.known_items.contains(item) { return None; }
        let mut found: Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = super::tconstruct_source_melting_recipes::material_melting(bytes, item, material, cost, conditions) else { continue; };
            let Some(primary) = self.resolved_fluid(&recipe.melting.output) else { continue; };
            let Some(byproducts) = recipe.byproducts.iter().map(|fluid| self.resolved_fluid(fluid).map(|id| (id, fluid.amount))).collect::<Option<Vec<_>>>() else { continue; };
            if let Some((_, old, old_primary, old_byproducts)) = &found {
                if old.melting.temperature != recipe.melting.temperature || old.melting.time != recipe.melting.time
                    || old.melting.output.amount != recipe.melting.output.amount || old_primary != &primary || old_byproducts != &byproducts { return None; }
            } else { found = Some((id.clone(), recipe, primary, byproducts)); }
        }
        found
    }
    pub(super) fn damaged_melting(&self, item: &str, damage: u32, maximum: u32, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        if !self.known_items.contains(item) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = super::tconstruct_source_melting_recipes::damaged_melting(bytes, damage, maximum, conditions) else { continue; };
            if !recipe.melting.input.matches(item, &in_tag) { continue; }
            let Some(primary) = self.resolved_fluid(&recipe.melting.output) else { continue; };
            let Some(byproducts) = recipe.byproducts.iter().map(|fluid| self.resolved_fluid(fluid).map(|id| (id, fluid.amount))).collect::<Option<Vec<_>>>() else { continue; };
            if let Some((_, old, old_primary, old_byproducts)) = &found {
                if old.melting.temperature != recipe.melting.temperature || old.melting.time != recipe.melting.time
                    || old.melting.output.amount != recipe.melting.output.amount || old_primary != &primary || old_byproducts != &byproducts { return None; }
            } else { found = Some((id.clone(), recipe, primary, byproducts)); }
        }
        found
    }
    pub(super) fn foundry_melting(&self, item: &str, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        self.foundry_melting_with_container(item, None, conditions)
    }
    pub(super) fn foundry_melting_with_container(&self, item: &str, no_container: Option<bool>, conditions: &impl Fn(&[Value]) -> bool)
        -> Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> {
        self.foundry_melting_with_rates(item, no_container, conditions, (Some(9), Some(4)), (Some(3), Some(4))).ok().flatten()
    }
    pub(super) fn foundry_melting_with_rates(&self, item: &str, no_container: Option<bool>, conditions: &impl Fn(&[Value]) -> bool,
        primary: (Option<u32>, Option<u32>), byproduct: (Option<u32>, Option<u32>))
        -> Result<Option<(String, FoundryRecipe, String, Vec<(String, u32)>)>, ()> {
        if !self.known_items.contains(item) { return Ok(None); }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, FoundryRecipe, String, Vec<(String, u32)>)> = None;
        for (id, bytes) in self.recipes {
            let Some(input) = MeltingRecipe::candidate_input(bytes, conditions) else { continue; };
            if !input.matches_with_container(item, &in_tag, no_container).ok_or(())? { continue; }
            if FoundryRecipe::parse_with_rates(bytes, conditions, (Some(9), Some(4)), (Some(9), Some(4))).is_none() { continue; }
            let recipe = FoundryRecipe::parse_with_rates(bytes, conditions, primary, byproduct).ok_or(())?;
            let Some(primary) = self.resolved_fluid(&recipe.melting.output) else { continue; };
            let Some(byproducts) = recipe.byproducts.iter().map(|fluid|
                self.resolved_fluid(fluid).map(|id| (id, fluid.amount))).collect::<Option<Vec<_>>>() else { continue; };
            if let Some((_, previous, old_primary, old_byproducts)) = &found {
                if previous.melting.temperature != recipe.melting.temperature || previous.melting.time != recipe.melting.time
                    || previous.melting.output.amount != recipe.melting.output.amount || old_primary != &primary
                    || old_byproducts != &byproducts { return Err(()); }
            } else { found = Some((id.clone(), recipe, primary, byproducts)); }
        }
        Ok(found)
    }
    pub(super) fn can_filling(&self, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe)> {
        if !self.known_items.contains("tconstruct:copper_can") || !self.known_fluids.contains(fluid) { return None; }
        let id = self.recipes.iter().find_map(|(id, bytes)| {
            let value: Value = serde_json::from_slice(bytes).ok()?;
            if value.get("type")?.as_str()? != "tconstruct:table_filling" || value.get("container")?.as_str()? != "tconstruct:copper_can" || value.get("fluid_amount")?.as_u64()? != 90 { return None; }
            match value.get("conditions") { None => {}, Some(Value::Array(values)) if conditions(values) => {}, _ => return None }
            Some(id.clone())
        })?;
        Some((id, CastingRecipe { basin: false, retextured: false, cast: Some(Ingredient::Item("tconstruct:copper_can".into())),
            consumes_cast: true, switch_slots: false, fluid: FluidSpec { id: fluid.to_owned(), tagged: false, amount: 90 },
            output: Output::Item("tconstruct:copper_can".into()), cooling_time: 5 }))
    }

    pub(super) fn portable_tank_filling(&self, basin: bool, input: &str, fluid: &str, free: u32, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) || free == 0 { return None; }
        let mut found: Option<(String, CastingRecipe)> = None;
        for (id, bytes) in self.recipes {
            let Some(source) = BasinFillingRecipe::parse(bytes, basin, conditions) else { continue; };
            if source.container != input { continue; }
            let amount = source.fluid_amount.min(free);
            let recipe = CastingRecipe { basin, retextured: false, cast: Some(Ingredient::Item(input.to_owned())),
                consumes_cast: true, switch_slots: false, fluid: FluidSpec { id: fluid.to_owned(), tagged: false, amount },
                output: Output::Item(input.to_owned()), cooling_time: 5 };
            if found.as_ref().is_some_and(|(_, old)| old != &recipe) { return None; }
            if found.is_none() { found = Some((id.clone(), recipe)); }
        }
        found
    }

    pub(super) fn bucket_filling(&self, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String)> {
        if !self.known_items.contains("minecraft:bucket") || !self.known_fluids.contains(fluid) { return None; }
        let output = super::tconstruct_source_fluid_container::filled_bucket(fluid)?;
        if !self.known_items.contains(&output) { return None; }
        let id = self.recipes.iter().find_map(|(id, bytes)| BucketFillingRecipe::matches(bytes, conditions).then(|| id.clone()))?;
        let recipe = CastingRecipe { basin: false, retextured: false, cast: Some(Ingredient::Item("minecraft:bucket".into())),
            consumes_cast: true, switch_slots: false, fluid: FluidSpec { id: fluid.to_owned(), tagged: false, amount: 1000 },
            output: Output::Item(output.clone()), cooling_time: 5 };
        Some((id, recipe, output))
    }

    pub(super) fn duplication(&self, basin: bool, input: &str, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe)> {
        if !self.known_items.contains(input) { return None; }
        let in_tag = |tag: &str, item: &str| self.item_tags.get(tag).is_some_and(|items| items.contains(item));
        let mut found: Option<(String, CastingRecipe)> = None;
        for (id, bytes) in self.recipes {
            let Some(duplicate) = DuplicationRecipe::parse(bytes, conditions) else { continue; };
            if duplicate.basin != basin || !duplicate.cast.matches(input, &in_tag) || !self.matches_fluid(&duplicate.fluid, fluid) { continue; }
            let recipe = CastingRecipe { basin, retextured: false, cast: Some(duplicate.cast), consumes_cast: false,
                switch_slots: false, fluid: duplicate.fluid, output: Output::Item(input.to_owned()), cooling_time: duplicate.cooling_time };
            if found.as_ref().is_some_and(|(_, old)| old != &recipe) { return None; }
            if found.is_none() { found = Some((id.clone(), recipe)); }
        }
        found
    }

    pub(super) fn casting(&self, basin: bool, input: Option<&str>, fluid: &str, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String)> {
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found: Option<(String, CastingRecipe, String)> = None;
        for (id, bytes) in self.recipes {
            let Some(recipe) = CastingRecipe::parse_for_fluid(bytes, conditions, &|spec| self.matches_fluid(spec, fluid)) else { continue; };
            if recipe.basin != basin || !self.matches_fluid(&recipe.fluid, fluid) { continue; }
            let matched = match (&recipe.cast, input) {
                (None, None) => true,
                (Some(cast), Some(item)) if self.known_items.contains(item) => cast.matches(item, &in_tag),
                _ => false,
            };
            if !matched { continue; }
            let Some(output) = self.output(&recipe.output) else { continue; };
            // The source retextures the result from the consumed cast. The
            // current controller models implement only these default brick
            // textures; reject other casts until their identities persist and
            // render correctly instead of silently losing their appearance.
            if recipe.retextured && !matches!((output.as_str(), input),
                ("tconstruct:smeltery_controller", Some("tconstruct:seared_bricks")) |
                ("tconstruct:foundry_controller", Some("tconstruct:scorched_bricks"))) { continue; }
            if let Some((_, previous, previous_output)) = &found {
                if previous.fluid.amount != recipe.fluid.amount || previous.cooling_time != recipe.cooling_time
                    || previous.consumes_cast != recipe.consumes_cast || previous.switch_slots != recipe.switch_slots || previous.retextured != recipe.retextured
                    || previous_output != &output || previous.output.potion() != recipe.output.potion() { return None; }
            } else { found = Some((id.clone(), recipe, output)); }
        }
        found
    }

    /// Composite casting transforms one material-bearing item of the same
    /// kind. Material variants are matched using the source wildcard rules.
    pub(super) fn composite_casting(&self, basin: bool, input: &str, input_material: &str, fluid: &str,
        supports: &impl Fn(&str, &str) -> bool,
        allowed: &impl Fn(Option<&Value>, &str) -> bool,
        stat_supported: &impl Fn(&str, &str) -> Option<bool>,
        conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String, String)> {
        if !self.known_items.contains(input) || !self.known_fluids.contains(fluid) { return None; }
        let split = |id: &str| { let (base, variant) = id.split_once('#').unwrap_or((id, "")); (base.to_owned(), variant.to_owned()) };
        let (input_base, input_variant) = split(input_material);
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(part) = CompositeCastingRecipe::parse(bytes, conditions) else { continue; };
            if part.basin != basin || part.result != input || !supports(input_material, input) { continue; }
            if let Some(stat) = part.casting_stat_conflict.as_deref() {
                let normal = self.recipes.values().filter_map(|bytes| MaterialFluidRecipe::parse(bytes, conditions))
                    .find(|recipe| self.matches_fluid(&recipe.fluid, fluid));
                if normal.as_ref().is_some_and(|recipe| stat_supported(&recipe.material, stat).unwrap_or(true)) { continue; }
            }
            for bytes in self.recipes.values() {
                let Some(material) = CompositeFluidRecipe::parse(bytes, conditions) else { continue; };
                let (wanted_base, wanted_variant) = split(&material.input);
                let (output_base, output_variant) = split(&material.material);
                let input_matches = wanted_base == input_base && (wanted_variant.is_empty() ||
                    wanted_variant == input_variant || wanted_variant == "default" && input_variant.is_empty());
                if !input_matches || (output_base == input_base && output_variant == input_variant)
                    || !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material, &part.result)
                    || !allowed(part.allowed_materials.as_ref(), &material.material) { continue; }
                let amount = material.fluid.amount.checked_mul(part.item_cost)?;
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: Some(Ingredient::Item(input.to_owned())),
                    consumes_cast: true, switch_slots: false, fluid: spec,
                    output: Output::Item(part.result.clone()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, part.result.clone(), material.material.clone());
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }

    /// Join the original part-cast and non-composite fluid-to-material recipes.
    /// Predicates are evaluated against the caller's world snapshot.
    pub(super) fn material_casting(&self, basin: bool, input: Option<&str>, fluid: &str, supports: &impl Fn(&str, &str) -> bool, allowed: &impl Fn(Option<&Value>, &str) -> bool, conditions: &impl Fn(&[Value]) -> bool) -> Option<(String, CastingRecipe, String, String)> {
        if input.is_some_and(|item| !self.known_items.contains(item)) || !self.known_fluids.contains(fluid) { return None; }
        let in_tag = |tag: &str, id: &str| self.item_tags.get(tag).is_some_and(|values| values.contains(id));
        let mut found = None;
        for (id, bytes) in self.recipes {
            let Some(part) = MaterialCastingRecipe::parse(bytes, conditions) else { continue; };
            let cast_matches = match (&part.cast, input) { (None, None) => true, (Some(cast), Some(item)) => cast.matches(item, &in_tag), _ => false };
            if part.basin != basin || !cast_matches || !self.known_items.contains(&part.result) { continue; }
            for fluid_bytes in self.recipes.values() {
                let Some(material) = MaterialFluidRecipe::parse(fluid_bytes, conditions) else { continue; };
                if !self.matches_fluid(&material.fluid, fluid) || !supports(&material.material, &part.result) || !allowed(part.allowed_materials.as_ref(), &material.material) { continue; }
                let Some(amount) = material.fluid.amount.checked_mul(part.item_cost) else { continue; };
                let factor = ((amount as f32) / 90.0_f32).sqrt();
                let time = (((material.temperature as f64 + 300.0).powf(0.585_f32 as f64)) * factor as f64).round();
                if !(1.0..=u32::MAX as f64).contains(&time) { continue; }
                let mut spec = material.fluid.clone(); spec.amount = amount;
                let recipe = CastingRecipe { basin, retextured: false, cast: part.cast.clone(),
                    consumes_cast: part.consumes_cast, switch_slots: part.switch_slots, fluid: spec,
                    output: Output::Item(part.result.clone()), cooling_time: time as u32 };
                let value = (id.clone(), recipe, part.result.clone(), material.material.clone());
                if found.as_ref().is_some_and(|old| old != &value) { return None; }
                found = Some(value);
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jeweled_apple_cast_requires_registered_result_and_matching_fluid() {
        let source = br#"{"type":"tconstruct:casting_table","cast":{"item":"minecraft:apple"},"cast_consumed":true,"cooling_time":118,"fluid":{"amount":200,"tag":"tconstruct:molten_diamond"},"result":"tconstruct:jeweled_apple"}"#.to_vec();
        let recipes = BTreeMap::from([("tconstruct:tools/modifiers/jeweled_apple".into(), source)]);
        let item_tags = HashMap::new();
        let fluid_tags = HashMap::from([("tconstruct:molten_diamond".into(), HashSet::from(["tconstruct:molten_diamond".into()]))]);
        let known_items = HashSet::from(["minecraft:apple".into(), "tconstruct:jeweled_apple".into()]);
        let known_fluids = HashSet::from(["tconstruct:molten_diamond".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let (_, recipe, result) = catalog.casting(false, Some("minecraft:apple"), "tconstruct:molten_diamond", &|_| true).unwrap();
        assert_eq!((result.as_str(), recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), ("tconstruct:jeweled_apple", 200, 118, true));
        assert!(catalog.casting(true, Some("minecraft:apple"), "tconstruct:molten_diamond", &|_| true).is_none());
        assert!(catalog.casting(false, None, "tconstruct:molten_diamond", &|_| true).is_none());
        assert!(catalog.casting(false, Some("minecraft:apple"), "minecraft:water", &|_| true).is_none());
        let missing_result = HashSet::from(["minecraft:apple".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &missing_result, known_fluids: &known_fluids };
        assert!(catalog.casting(false, Some("minecraft:apple"), "tconstruct:molten_diamond", &|_| true).is_none());
    }
    #[test]
    fn mighty_smelting_book_uses_the_source_table_recipe() {
        let source = br#"{"type":"tconstruct:casting_table","cast":{"item":"minecraft:book"},"cast_consumed":true,"cooling_time":89,"fluid":{"amount":250,"tag":"tconstruct:seared_stone"},"result":"tconstruct:mighty_smelting"}"#.to_vec();
        let recipes = BTreeMap::from([("tconstruct:common/mighty_smelting".into(), source)]);
        let item_tags = HashMap::new();
        let fluid_tags = HashMap::from([("tconstruct:seared_stone".into(), HashSet::from(["tconstruct:seared_stone".into()]))]);
        let known_items = HashSet::from(["minecraft:book".into(), "tconstruct:mighty_smelting".into()]);
        let known_fluids = HashSet::from(["tconstruct:seared_stone".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let (_, recipe, result) = catalog.casting(false, Some("minecraft:book"), "tconstruct:seared_stone", &|_| true).unwrap();
        assert_eq!((result.as_str(), recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), ("tconstruct:mighty_smelting", 250, 89, true));
        assert!(catalog.casting(true, Some("minecraft:book"), "tconstruct:seared_stone", &|_| true).is_none());
        assert!(catalog.casting(false, Some("minecraft:paper"), "tconstruct:seared_stone", &|_| true).is_none());
        assert!(catalog.casting(false, Some("minecraft:book"), "minecraft:water", &|_| true).is_none());
    }
    #[test]
    fn all_catalyst_alloy_requires_capacity_without_consuming_input() {
        let recipe = br#"{"type":"tconstruct:alloy","inputs":[{"fluid":"minecraft:water","amount":250,"catalyst":true},{"fluid":"minecraft:lava","amount":100,"catalyst":true}],"result":{"fluid":"test:result","amount":100},"temperature":700}"#.to_vec();
        let recipes = std::collections::BTreeMap::from([("catalyst".into(), recipe)]);
        let tags = HashMap::new(); let items = HashSet::new();
        let fluids = HashSet::from(["minecraft:water".into(), "minecraft:lava".into(), "test:result".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let available = [("minecraft:water".into(), 250), ("minecraft:lava".into(), 100)];
        assert!(catalog.alloy(&available, 449, 700, &|_| true).is_none());
        let (_, consumed, _, amount) = catalog.alloy(&available, 450, 700, &|_| true).unwrap();
        assert!(consumed.is_empty()); assert_eq!(amount, 100);
        assert!(catalog.alloy(&[("minecraft:water".into(), 249), ("minecraft:lava".into(), 100)], 450, 700, &|_| true).is_none());
    }

    #[test]
    fn alloy_rejects_tagged_matching_inputs() {
        let recipe = br#"{"type":"tconstruct:alloy","inputs":[{"amount":90,"fluid":"test:copper"},{"amount":90,"fluid":"test:tin"}],"result":{"amount":180,"fluid":"test:bronze"},"temperature":700}"#.to_vec();
        let recipes = std::collections::BTreeMap::from([("alloy".into(), recipe)]);
        let tags = HashMap::new(); let items = HashSet::new();
        let fluids = HashSet::from(["test:copper".into(), "test:tin".into(), "test:bronze".into(), "test:potion".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let available = [("test:copper".into(), 90), ("test:tin".into(), 90)];
        assert!(catalog.alloy_except(&available, &["test:potion".into()], 180, 700, &|_| true).is_some());
        assert!(catalog.alloy_except(&available, &["test:copper".into()], 180, 700, &|_| true).is_none());
    }

    #[test]
    fn configured_rates_use_matched_recipe_and_unknown_does_not_fall_through() {
        let ore = br#"{"type":"tconstruct:ore_melting","ingredient":{"item":"minecraft:copper_ore"},"result":{"amount":180,"fluid":"tconstruct:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"amount":90,"fluid":"tconstruct:molten_gold"}]}"#.to_vec();
        let ordinary = br#"{"type":"tconstruct:melting","ingredient":{"item":"minecraft:iron_ingot"},"result":{"amount":90,"fluid":"tconstruct:molten_iron"},"temperature":500,"time":125}"#.to_vec();
        let recipes = BTreeMap::from([("a".into(), ore), ("b".into(), ordinary)]);
        let tags = HashMap::new();
        let items = HashSet::from(["minecraft:copper_ore".into(), "minecraft:iron_ingot".into()]);
        let fluids = HashSet::from(["tconstruct:molten_copper".into(), "tconstruct:molten_gold".into(), "tconstruct:molten_iron".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let result = catalog.melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(18), Some(8))).unwrap().unwrap();
        assert_eq!(result.1.output.amount, 360);
        assert!(catalog.melting_with_rates("minecraft:copper_ore", None, &|_| true, (None, Some(8))).is_err());
        assert_eq!(catalog.melting_with_rates("minecraft:iron_ingot", None, &|_| true, (None, None)).unwrap().unwrap().1.output.amount, 90);
        let result = catalog.foundry_melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(12), Some(4)), (Some(6), Some(4))).unwrap().unwrap();
        assert_eq!((result.1.melting.output.amount, result.3[0].1), (240, 60));
        assert!(catalog.foundry_melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(9), Some(4)), (None, Some(4))).is_err());
    }

    #[test]
    fn malformed_matching_recipe_does_not_shadow_valid_recipe() {
        let bad = br#"{"type":"tconstruct:melting","ingredient":{"item":"minecraft:iron_ingot"},"result":{"amount":90,"fluid":"tconstruct:molten_iron"}}"#.to_vec();
        let good = br#"{"type":"tconstruct:melting","ingredient":{"item":"minecraft:iron_ingot"},"result":{"amount":90,"fluid":"tconstruct:molten_iron"},"temperature":500,"time":125}"#.to_vec();
        let recipes = BTreeMap::from([("a".into(), bad), ("b".into(), good)]);
        let tags = HashMap::new();
        let items = HashSet::from(["minecraft:iron_ingot".into()]);
        let fluids = HashSet::from(["tconstruct:molten_iron".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        assert_eq!(catalog.melting_with_rates("minecraft:iron_ingot", None, &|_| true, (None, None)).unwrap().unwrap().0, "b");
        assert_eq!(catalog.foundry_melting_with_rates("minecraft:iron_ingot", None, &|_| true, (None, None), (None, None)).unwrap().unwrap().0, "b");
    }

    #[test]
    fn foundry_tiny_byproduct_does_not_preparse_at_default_rate() {
        let ore = br#"{"type":"tconstruct:ore_melting","ingredient":{"item":"minecraft:copper_ore"},"result":{"amount":90,"fluid":"tconstruct:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"amount":1,"fluid":"tconstruct:molten_gold"}]}"#.to_vec();
        let recipes = BTreeMap::from([("ore".into(), ore)]);
        let tags = HashMap::new();
        let items = HashSet::from(["minecraft:copper_ore".into()]);
        let fluids = HashSet::from(["tconstruct:molten_copper".into(), "tconstruct:molten_gold".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let full = catalog.foundry_melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(9), Some(4)), (Some(9), Some(4))).unwrap().unwrap();
        assert_eq!((full.1.melting.output.amount, full.3[0].1), (90, 1));
        let zero = catalog.foundry_melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(9), Some(4)), (Some(1), Some(4))).unwrap().unwrap();
        assert_eq!(zero.1.melting.output.amount, 90);
        assert!(zero.3.is_empty());
    }

    #[test]
    fn configured_low_rate_is_not_prechecked_with_overflowing_default() {
        let ore = br#"{"type":"tconstruct:ore_melting","ingredient":{"item":"minecraft:copper_ore"},"result":{"amount":400000000,"fluid":"tconstruct:molten_copper"},"temperature":500,"time":125,"rate":"metal"}"#.to_vec();
        let recipes = BTreeMap::from([("ore".into(), ore)]);
        let tags = HashMap::new();
        let items = HashSet::from(["minecraft:copper_ore".into()]);
        let fluids = HashSet::from(["tconstruct:molten_copper".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let result = catalog.melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(1), Some(8))).unwrap().unwrap();
        assert_eq!(result.1.output.amount, 400000000 / 9);
        let foundry = catalog.foundry_melting_with_rates("minecraft:copper_ore", None, &|_| true, (Some(1), Some(4)), (Some(3), Some(4))).unwrap().unwrap();
        assert_eq!(foundry.1.melting.output.amount, 400000000 / 9);
    }

    #[test]
    fn foundry_resolves_primary_and_byproduct_tags() {
        let recipe = br#"{"type":"tconstruct:ore_melting","ingredient":{"tag":"forge:ores/copper"},"result":{"amount":180,"tag":"forge:molten_copper"},"temperature":500,"time":125,"rate":"metal","byproducts":[{"amount":90,"tag":"forge:molten_gold"}]}"#;
        let recipes = [("ore".into(), recipe.to_vec())].into_iter().collect();
        let item_tags = [("forge:ores/copper".into(), ["minecraft:copper_ore".into()].into_iter().collect())].into_iter().collect();
        let fluid_tags = [("forge:molten_copper".into(), ["tconstruct:molten_copper".into()].into_iter().collect()),
            ("forge:molten_gold".into(), ["tconstruct:molten_gold".into()].into_iter().collect())].into_iter().collect();
        let known_items = ["minecraft:copper_ore".into()].into_iter().collect();
        let known_fluids = ["tconstruct:molten_copper".into(), "tconstruct:molten_gold".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let (_, recipe, primary, byproducts) = catalog.foundry_melting("minecraft:copper_ore", &|_| true).unwrap();
        assert_eq!((primary.as_str(), recipe.melting.output.amount), ("tconstruct:molten_copper", 180));
        assert_eq!(byproducts, vec![("tconstruct:molten_gold".into(), 30)]);
    }
    #[test]
    fn alloy_resolves_source_tags_and_temperature() {
        let recipes = [
            ("a-overflow".into(), br#"{"type":"tconstruct:alloy","inputs":[{"amount":270,"tag":"forge:copper"},{"amount":90,"tag":"forge:tin"}],"result":{"amount":361,"tag":"forge:bronze"},"temperature":700}"#.to_vec()),
            ("bronze".into(), br#"{"type":"tconstruct:alloy","inputs":[{"amount":270,"tag":"forge:copper"},{"amount":90,"tag":"forge:tin"}],"result":{"amount":360,"tag":"forge:bronze"},"temperature":700}"#.to_vec())
        ].into_iter().collect();
        let item_tags = HashMap::new(); let items = HashSet::new();
        let fluids = ["tconstruct:copper".into(), "tconstruct:tin".into(), "tconstruct:bronze".into()].into_iter().collect();
        let tags = [("forge:copper".into(), ["tconstruct:copper".into()].into_iter().collect()), ("forge:tin".into(), ["tconstruct:tin".into()].into_iter().collect()), ("forge:bronze".into(), ["tconstruct:bronze".into()].into_iter().collect())].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let stored = [("tconstruct:copper".into(), 270), ("tconstruct:tin".into(), 90)];
        assert!(catalog.alloy(&stored, 360, 699, &|_| true).is_none());
        let (id, inputs, output, amount) = catalog.alloy(&stored, 360, 700, &|_| true).unwrap();
        assert_eq!(id, "bronze");
        assert_eq!((inputs.len(), output.as_str(), amount), (2, "tconstruct:bronze", 360));
        assert!(catalog.alloy(&stored[..1], 360, 700, &|_| true).is_none());
    }
    #[test]
    fn duplication_matches_template_and_fluid_without_consuming_cast() {
        let recipes = [("template".into(), br#"{"type":"tconstruct:table_duplication","cast":{"tag":"minecraft:trim_templates"},"fluid":{"tag":"tconstruct:molten_diamond","amount":500},"cooling_time":186}"#.to_vec())].into_iter().collect();
        let item_tags = [("minecraft:trim_templates".into(), ["minecraft:coast_armor_trim_smithing_template".into()].into_iter().collect())].into_iter().collect();
        let fluid_tags = [("tconstruct:molten_diamond".into(), ["tconstruct:molten_diamond".into()].into_iter().collect())].into_iter().collect();
        let items = ["minecraft:coast_armor_trim_smithing_template".into(), "minecraft:stone".into()].into_iter().collect();
        let fluids = ["tconstruct:molten_diamond".into(), "minecraft:water".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        let (_, recipe) = catalog.duplication(false, "minecraft:coast_armor_trim_smithing_template", "tconstruct:molten_diamond", &|_| true).unwrap();
        assert!(!recipe.consumes_cast);
        assert_eq!((recipe.fluid.amount, recipe.cooling_time), (500, 186));
        assert!(catalog.duplication(false, "minecraft:stone", "tconstruct:molten_diamond", &|_| true).is_none());
        assert!(catalog.duplication(false, "minecraft:coast_armor_trim_smithing_template", "minecraft:water", &|_| true).is_none());
        assert!(catalog.duplication(true, "minecraft:coast_armor_trim_smithing_template", "tconstruct:molten_diamond", &|_| true).is_none());
    }
    #[test]
    fn table_filling_requires_registered_bucket_and_fluid() {
        let recipes = [("bucket".into(), br#"{"type":"tconstruct:table_filling","container":"minecraft:bucket","fluid_amount":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["minecraft:bucket".into(), "minecraft:lava_bucket".into()].into_iter().collect();
        let fluids = ["minecraft:lava".into(), "tconstruct:venom".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        let (_, recipe, output) = catalog.bucket_filling("minecraft:lava", &|_| true).unwrap();
        assert_eq!((recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), (1000, 5, true));
        assert_eq!(output, "minecraft:lava_bucket");
        assert!(catalog.bucket_filling("tconstruct:venom", &|_| true).is_none());
        assert!(catalog.bucket_filling("minecraft:water", &|_| true).is_none());
    }
    #[test]
    fn table_filling_uses_flowing_fluid_identity_and_shared_bucket() {
        let recipes = [("bucket".into(), br#"{"type":"tconstruct:table_filling","container":"minecraft:bucket","fluid_amount":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["minecraft:bucket".into(), "minecraft:water_bucket".into(), "tconstruct:molten_copper_bucket".into()].into_iter().collect();
        let fluids = ["minecraft:flowing_water".into(), "minecraft:flowing_lava".into(), "tconstruct:flowing_molten_copper".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        for (fluid, bucket) in [("minecraft:flowing_water", "minecraft:water_bucket"), ("tconstruct:flowing_molten_copper", "tconstruct:molten_copper_bucket")] {
            let (_, recipe, output) = catalog.bucket_filling(fluid, &|_| true).unwrap();
            assert_eq!(recipe.fluid.id, fluid);
            assert_eq!((recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), (1000, 5, true));
            assert_eq!(output, bucket);
        }
        assert!(catalog.bucket_filling("minecraft:flowing_lava", &|_| true).is_none());
        assert!(catalog.bucket_filling("tconstruct:flowing_unknown", &|_| true).is_none());
    }
    #[test]
    fn table_filling_maps_powdered_snow_to_vanilla_bucket() {
        let recipes = [("bucket".into(), br#"{"type":"tconstruct:table_filling","container":"minecraft:bucket","fluid_amount":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["minecraft:bucket".into(), "minecraft:powder_snow_bucket".into()].into_iter().collect();
        let fluids = ["tconstruct:powdered_snow".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        let (_, recipe, output) = catalog.bucket_filling("tconstruct:powdered_snow", &|_| true).unwrap();
        assert_eq!((recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), (1000, 5, true));
        assert_eq!(output, "minecraft:powder_snow_bucket");
    }
    #[test]
    fn alloy_catalyst_requires_fluid_but_does_not_consume_it() {
        let recipes = [("obsidian".into(), br#"{"type":"tconstruct:alloy","inputs":[{"fluid":"minecraft:water","amount":250,"catalyst":true},{"fluid":"minecraft:lava","amount":100}],"result":{"fluid":"tconstruct:molten_obsidian","amount":100},"temperature":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let items = HashSet::new(); let tags = HashMap::new();
        let fluids = ["minecraft:water".into(), "minecraft:lava".into(), "tconstruct:molten_obsidian".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let available = [("minecraft:water".into(), 250), ("minecraft:lava".into(), 100)];
        let (_, inputs, output, amount) = catalog.alloy(&available, 350, 1000, &|_| true).unwrap();
        assert_eq!(inputs, vec![("minecraft:lava".into(), 100)]);
        assert_eq!((output.as_str(), amount), ("tconstruct:molten_obsidian", 100));
        assert!(catalog.alloy(&available[1..], 350, 1000, &|_| true).is_none());
        assert!(catalog.alloy(&available, 350, 999, &|_| true).is_none());
    }
    #[test]
    fn basin_material_casting_requires_empty_cast_and_scales_cost() {
        let recipes = [
            ("block".into(), br#"{"type":"tconstruct:basin_casting_material","item_cost":9,"result":"tconstruct:fake_storage_block"}"#.to_vec()),
            ("fluid".into(), br#"{"type":"tconstruct:material_fluid","fluid":{"fluid":"tconstruct:molten_test","amount":90},"output":"tconstruct:test","temperature":500}"#.to_vec()),
        ].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["tconstruct:fake_storage_block".into(), "minecraft:stone".into()].into_iter().collect();
        let fluids = ["tconstruct:molten_test".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        let (_, recipe, result, material) = catalog.material_casting(true, None, "tconstruct:molten_test", &|_, _| true, &|_, _| true, &|_| true).unwrap();
        assert_eq!((recipe.fluid.amount, result.as_str(), material.as_str()), (810, "tconstruct:fake_storage_block", "tconstruct:test"));
        assert!(catalog.material_casting(false, None, "tconstruct:molten_test", &|_, _| true, &|_, _| true, &|_| true).is_none());
        assert!(catalog.material_casting(true, Some("minecraft:stone"), "tconstruct:molten_test", &|_, _| true, &|_, _| true, &|_| true).is_none());
        assert!(catalog.material_casting(true, None, "tconstruct:molten_test", &|_, _| false, &|_, _| true, &|_| true).is_none());
    }
    #[test]
    fn basin_filling_respects_recipe_and_remaining_capacity() {
        let recipes = [("tank".into(), br#"{"type":"tconstruct:basin_filling","container":"tconstruct:seared_fuel_tank","fluid_amount":250}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["tconstruct:seared_fuel_tank".into(), "minecraft:stone".into()].into_iter().collect();
        let fluids = ["minecraft:lava".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        let (_, recipe) = catalog.portable_tank_filling(true, "tconstruct:seared_fuel_tank", "minecraft:lava", 50, &|_| true).unwrap();
        assert_eq!((recipe.fluid.amount, recipe.cooling_time, recipe.consumes_cast), (50, 5, true));
        assert!(catalog.portable_tank_filling(true, "tconstruct:seared_fuel_tank", "minecraft:lava", 0, &|_| true).is_none());
        assert!(catalog.portable_tank_filling(true, "minecraft:stone", "minecraft:lava", 250, &|_| true).is_none());
        assert!(catalog.portable_tank_filling(true, "tconstruct:seared_fuel_tank", "minecraft:water", 250, &|_| true).is_none());
    }
    #[test]
    fn lantern_table_and_basin_use_distinct_source_quantities() {
        let recipes = [
            ("table".into(), br#"{"type":"tconstruct:table_filling","container":"tconstruct:seared_lantern","fluid_amount":10}"#.to_vec()),
            ("basin".into(), br#"{"type":"tconstruct:basin_filling","container":"tconstruct:seared_lantern","fluid_amount":50}"#.to_vec()),
        ].into_iter().collect();
        let item_tags = HashMap::new(); let fluid_tags = HashMap::new();
        let items = ["tconstruct:seared_lantern".into()].into_iter().collect();
        let fluids = ["minecraft:lava".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        assert_eq!(catalog.portable_tank_filling(false, "tconstruct:seared_lantern", "minecraft:lava", 50, &|_| true).unwrap().1.fluid.amount, 10);
        assert_eq!(catalog.portable_tank_filling(true, "tconstruct:seared_lantern", "minecraft:lava", 50, &|_| true).unwrap().1.fluid.amount, 50);
        assert_eq!(catalog.portable_tank_filling(true, "tconstruct:seared_lantern", "minecraft:lava", 7, &|_| true).unwrap().1.fluid.amount, 7);
        assert!(catalog.portable_tank_filling(true, "tconstruct:seared_lantern", "minecraft:lava", 0, &|_| true).is_none());
    }
    #[test]
    fn fluid_fuel_requires_known_fluid_and_enough_amount() {
        let recipes = [("lava".into(), br#"{"type":"tconstruct:melting_fuel","duration":100,"fluid":{"amount":50,"fluid":"minecraft:lava"},"rate":10,"temperature":1000}"#.to_vec())].into_iter().collect();
        let item_tags = HashMap::new(); let items = HashSet::new(); let tags = HashMap::new();
        let fluids = ["minecraft:lava".into()].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        assert!(catalog.fluid_fuel("minecraft:lava", 49, &|_| true).is_none());
        assert_eq!(catalog.fluid_fuel("minecraft:lava", 50, &|_| true).unwrap().duration, 100);
        assert!(catalog.fluid_fuel("minecraft:water", 50, &|_| true).is_none());
    }
    #[test]
    fn copper_round_trip_uses_resolved_tags_and_rejects_ambiguous_outputs() {
        let recipes = [
            ("melting".into(), br#"{"type":"tconstruct:melting","ingredient":{"tag":"forge:ingots/copper"},"result":{"amount":90,"tag":"forge:molten_copper"},"temperature":500,"time":50}"#.to_vec()),
            ("casting".into(), br#"{"type":"tconstruct:casting_table","cast":{"tag":"tconstruct:casts/multi_use/ingot"},"cooling_time":50,"fluid":{"amount":90,"tag":"forge:molten_copper"},"result":{"tag":"forge:ingots/copper"}}"#.to_vec()),
        ].into_iter().collect();
        let items: HashSet<String> = ["minecraft:copper_ingot".into(), "tconstruct:ingot_cast".into()].into_iter().collect();
        let fluids: HashSet<String> = ["tconstruct:molten_copper".into(), "tconstruct:flowing_molten_copper".into()].into_iter().collect();
        let mut item_tags: HashMap<String, HashSet<String>> = [
            ("forge:ingots/copper".into(), ["minecraft:copper_ingot".into()].into_iter().collect()),
            ("tconstruct:casts/multi_use/ingot".into(), ["tconstruct:ingot_cast".into()].into_iter().collect()),
        ].into_iter().collect();
        let fluid_tags = [("forge:molten_copper".into(), fluids.clone())].into_iter().collect();
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &items, known_fluids: &fluids };
        assert_eq!(catalog.melting("minecraft:copper_ingot", &|_| true).unwrap().2, "tconstruct:molten_copper");
        assert_eq!(catalog.casting(false, Some("tconstruct:ingot_cast"), "tconstruct:molten_copper", &|_| true).unwrap().2, "minecraft:copper_ingot");
        assert!(catalog.casting(false, None, "tconstruct:molten_copper", &|_| true).is_none());
        item_tags.get_mut("forge:ingots/copper").unwrap().insert("other:copper_ingot".into());
        let mut ambiguous_items = items.clone(); ambiguous_items.insert("other:copper_ingot".into());
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &ambiguous_items, known_fluids: &fluids };
        assert!(catalog.casting(false, Some("tconstruct:ingot_cast"), "tconstruct:molten_copper", &|_| true).is_none());
    }
}

/// Resolve the source pack's tank-fluid tags without registering flowing
/// world fluids. These IDs describe contents held by the melter; world-fluid
/// placement remains separate. Only source TConstruct IDs and the native
/// water/lava and Forge milk families are admitted; unrelated mods are not assumed present.
pub(super) fn tank_fluid_tags(
    raw: &BTreeMap<String, Vec<Vec<u8>>>,
    source_fluids: &HashSet<String>,
) -> Result<(HashSet<String>, HashMap<String, HashSet<String>>), String> {
    let mut fluids: HashSet<String> = ["minecraft:empty", "minecraft:water", "minecraft:flowing_water", "minecraft:lava", "minecraft:flowing_lava", "minecraft:milk", "minecraft:flowing_milk"]
        .into_iter().map(str::to_owned).collect();
    // Only IDs from the retained upstream catalog are virtual tank contents;
    // a datapack tag cannot invent a registered fluid by itself.
    fluids.extend(source_fluids.iter().filter(|id| id.starts_with("tconstruct:")).cloned());
    let tags = super::tconstruct_source_tags::resolve_registry(raw, &fluids)?;
    Ok((fluids, tags))
}

#[cfg(test)]
mod fluid_tag_tests {
    use super::*;
    #[test]
    fn forge_milk_tags_enable_head_melting_and_cheese_casting() {
        // Pinned Forge 47.4.0 milk tag and TConstruct f4e77d43 recipes.
        let raw = [("forge:milk".into(), vec![br#"{"values":[{"id":"minecraft:milk","required":false},{"id":"minecraft:flowing_milk","required":false}]}"#.to_vec()])].into_iter().collect();
        let (fluids, tags) = tank_fluid_tags(&raw, &HashSet::new()).unwrap();
        assert_eq!(tags["forge:milk"], HashSet::from(["minecraft:milk".into(), "minecraft:flowing_milk".into()]));
        let recipes = [
            ("head".into(), br#"{"type":"tconstruct:melting","ingredient":[{"item":"minecraft:skeleton_skull"},{"item":"minecraft:wither_skeleton_skull"},{"item":"tconstruct:stray_head"}],"result":{"amount":250,"fluid":"minecraft:milk"},"temperature":0,"time":47}"#.to_vec()),
            ("cheese".into(), br#"{"type":"tconstruct:casting_basin","cooling_time":6000,"fluid":{"amount":1000,"tag":"forge:milk"},"result":"tconstruct:cheese_block"}"#.to_vec()),
        ].into_iter().collect();
        let item_tags = HashMap::new();
        let items = HashSet::from(["minecraft:skeleton_skull".into(), "tconstruct:cheese_block".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let (_, melt, fluid) = catalog.melting("minecraft:skeleton_skull", &|_| true).unwrap();
        assert_eq!((fluid.as_str(), melt.output.amount), ("minecraft:milk", 250));
        for fluid in ["minecraft:milk", "minecraft:flowing_milk"] {
            let (_, recipe, result) = catalog.casting(true, None, fluid, &|_| true).unwrap();
            assert_eq!((result.as_str(), recipe.fluid.amount), ("tconstruct:cheese_block", 1000));
        }
        assert!(catalog.casting(true, None, "other:milk", &|_| true).is_none());
    }
    #[test]
    fn resolves_source_copper_without_assuming_other_mod_fluids() {
        let raw = [
            ("forge:molten_copper".into(), vec![br##"{"values":["#tconstruct:molten_copper"]}"##.to_vec()]),
            ("tconstruct:molten_copper".into(), vec![br#"{"values":["tconstruct:molten_copper","tconstruct:flowing_molten_copper"]}"#.to_vec()]),
            ("forge:external".into(), vec![br#"{"values":["other:fluid"]}"#.to_vec()]),
        ].into_iter().collect();
        let source_fluids = ["tconstruct:molten_copper".into(), "tconstruct:flowing_molten_copper".into()].into_iter().collect();
        let (fluids, tags) = tank_fluid_tags(&raw, &source_fluids).unwrap();
        assert!(fluids.contains("tconstruct:molten_copper"));
        assert!(!fluids.contains("other:fluid"));
        assert!(tags["forge:molten_copper"].contains("tconstruct:molten_copper"));
        assert!(!tags.contains_key("forge:external"));
    }
}

#[cfg(test)]
mod slimeskull_tests {
    use super::*;
    #[test]
    fn consumed_offset_uses_skull_then_slime_and_source_cost() {
        let skull = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:skeleton_skull"},"cast_purpose":"consumed_offset","extra_materials":["tconstruct:bone"],"fluid_swapping":false,"item_cost":5,"result":"tconstruct:slime_helmet"}"#;
        let fluid = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"tag":"forge:slime"},"output":"tconstruct:earthslime","temperature":50}"#;
        let recipes = BTreeMap::from([("skull".into(), skull.to_vec()), ("fluid".into(), fluid.to_vec())]);
        let item_tags = HashMap::new();
        let fluid_tags = HashMap::from([("forge:slime".into(), HashSet::from(["tconstruct:earth_slime".into()]))]);
        let known_items = HashSet::from(["minecraft:skeleton_skull".into(), "tconstruct:slime_helmet".into()]);
        let known_fluids = HashSet::from(["tconstruct:earth_slime".into()]);
        let c = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let supports = |id: &str, kind: &str| (id, kind) == ("tconstruct:bone", "tconstruct:skull") || (id, kind) == ("tconstruct:earthslime", "tconstruct:slime");
        let (_, recipe, skull, slime) = c.slimeskull_casting(true, "minecraft:skeleton_skull", "tconstruct:earth_slime", &supports, &|_| true).unwrap();
        assert_eq!((skull.as_str(), slime.as_str(), recipe.fluid.amount, recipe.consumes_cast), ("tconstruct:bone", "tconstruct:earthslime", 1250, true));
        assert!(recipe.cooling_time > 0);
        assert!(c.slimeskull_casting(false, "minecraft:skeleton_skull", "tconstruct:earth_slime", &supports, &|_| true).is_none());
        assert!(c.slimeskull_casting(true, "minecraft:other", "tconstruct:earth_slime", &supports, &|_| true).is_none());
        assert!(c.slimeskull_casting(true, "minecraft:skeleton_skull", "tconstruct:earth_slime", &|_,_| false, &|_| true).is_none());
    }
    #[test]
    fn rejects_unhandled_recipe_semantics() {
        let recipe = |extra: &str| format!(r#"{{"type":"tconstruct:basin_tool_casting","cast":{{"item":"minecraft:skeleton_skull"}},"cast_purpose":"consumed_offset","extra_materials":["tconstruct:bone"],"fluid_swapping":false,"item_cost":5,"result":"tconstruct:slime_helmet"{extra}}}"#);
        let parse = |value: &str| super::super::tconstruct_source_melting_recipes::SlimeskullCastingRecipe::parse(value.as_bytes(), &|_| true);
        assert!(parse(&recipe("")).is_some());
        assert!(parse(&recipe(r#", "allowed_materials":{"type":"example:unknown"}"#)).is_none());
        assert!(parse(&recipe(r#", "extra_materials":["a","b"]"#)).is_none());
    }
    #[test]
    fn overlapping_supported_tool_casts_fail_closed() {
        let skull = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed_offset","extra_materials":["tconstruct:bone"],"fluid_swapping":false,"item_cost":5,"result":"tconstruct:slime_helmet"}"#;
        let wings = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed","item_cost":8,"result":"tconstruct:slime_wings"}"#;
        let recipes = BTreeMap::from([("skull".into(), skull.to_vec()), ("wings".into(), wings.to_vec())]);
        let tags = HashMap::new();
        let items = HashSet::from(["minecraft:elytra".into(), "tconstruct:slime_helmet".into(), "tconstruct:slime_wings".into()]);
        let fluids = HashSet::from(["test:fluid".into()]);
        let c = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        assert!(c.slimeskull_casting(true, "minecraft:elytra", "test:fluid", &|_,_| true, &|_| true).is_none());
        assert!(c.slimelytra_casting(true, "minecraft:elytra", "test:fluid", &|_| true, &|_| true).is_none());
    }
}

#[cfg(test)]
mod part_swap_predicate_tests {
    use super::*;
#[test]
fn swap_filters_output_material_and_preserves_composite_fallback() {
    let swap = br#"{"type":"tconstruct:table_casting_part_swapping","tools":{"item":"tconstruct:tool"},"item_cost":2,"index":0,"allowed_materials":{"type":"tconstruct:id","id":"tconstruct:bronze"}}"#;
    let normal = br#"{"type":"tconstruct:material_fluid","output":"tconstruct:iron","temperature":1000,"fluid":{"fluid":"test:metal","amount":90}}"#;
    let composite = br#"{"type":"tconstruct:material_fluid","input":"tconstruct:copper","output":"tconstruct:bronze","temperature":1000,"fluid":{"fluid":"test:metal","amount":90}}"#;
    let recipes: BTreeMap<String, Vec<u8>> = [("swap", swap.as_slice()), ("normal", normal.as_slice()), ("composite", composite.as_slice())].into_iter().map(|(k,v)| (k.into(), v.into())).collect();
    let tags = HashMap::new();
    let items = HashSet::from(["tconstruct:tool".into()]);
    let fluids = HashSet::from(["test:metal".into()]);
    let c = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
    let allowed = |v: Option<&Value>, m: &str| material_predicate(v,m,&|_,_| false,&|_,_| None,&|_,_,_| None).unwrap_or(false);
    let conditions = |_: &[Value]| true;
    assert!(c.part_swap_casting(false,"tconstruct:tool","test:metal", &|_,_| true,&allowed,&conditions).is_none());
    let identities = vec!["tconstruct:copper".into()];
    let (_, recipe, material, index) = c.composite_part_swap(false,"tconstruct:tool","test:metal",&identities,&|_,_| true,&|_,_| true,&allowed,&conditions).unwrap();
    assert_eq!((material.as_str(),index,recipe.fluid.amount), ("tconstruct:bronze",0,180));
    assert!(c.composite_part_swap(false,"tconstruct:tool","test:metal",&identities,&|_,_| true,&|_,_| true,&|_,_| false,&conditions).is_none());
}

#[test]
fn same_material_composite_is_allowed_when_ordinary_is_not() {
    let swap = br#"{"type":"tconstruct:table_casting_part_swapping","tools":{"item":"tconstruct:tool"},"item_cost":2,"index":0}"#;
    let normal = br#"{"type":"tconstruct:material_fluid","output":"tconstruct:copper","temperature":1000,"fluid":{"fluid":"test:metal","amount":90}}"#;
    let composite = br#"{"type":"tconstruct:material_fluid","input":"tconstruct:copper","output":"tconstruct:copper","temperature":1000,"fluid":{"fluid":"test:metal","amount":90}}"#;
    let recipes: BTreeMap<String, Vec<u8>> = [("swap", swap.as_slice()), ("normal", normal.as_slice()), ("composite", composite.as_slice())].into_iter().map(|(k,v)| (k.into(),v.into())).collect();
    let tags = HashMap::new(); let items = HashSet::from(["tconstruct:tool".into()]); let fluids = HashSet::from(["test:metal".into()]);
    let c = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
    let identities = vec!["tconstruct:copper".into()];
    let ordinary = |_: usize, material: &str| material != "tconstruct:copper";
    assert!(c.part_swap_casting(false,"tconstruct:tool","test:metal",&ordinary,&|_,_| true,&|_| true).is_none());
    let (_, recipe, material, _) = c.composite_part_swap(false,"tconstruct:tool","test:metal",&identities,&|_,_| true,&ordinary,&|_,_| true,&|_| true).unwrap();
    assert_eq!((material.as_str(),recipe.fluid.amount),("tconstruct:copper",180));
    assert!(c.composite_part_swap(false,"tconstruct:tool","test:metal",&identities,&|_,_| false,&ordinary,&|_,_| true,&|_| true).is_none());
}

#[test]
fn normal_swap_respects_optional_and_unknown_predicates() {
    let normal = br#"{"type":"tconstruct:material_fluid","output":"tconstruct:iron","temperature":1000,"fluid":{"fluid":"test:metal","amount":90}}"#;
    let tags = HashMap::new(); let items = HashSet::from(["tconstruct:tool".into()]); let fluids = HashSet::from(["test:metal".into()]);
    for (predicate, expected) in [("", true), (r#", "allowed_materials":{"type":"tconstruct:id","id":"tconstruct:iron"}"#, true),
        (r#", "allowed_materials":{"type":"tconstruct:id","id":"tconstruct:copper"}"#, false),
        (r#", "allowed_materials":{"type":"unknown:predicate"}"#, false)] {
        let swap = format!(r#"{{"type":"tconstruct:basin_casting_part_swapping","tools":{{"item":"tconstruct:tool"}},"item_cost":2,"index":0{predicate}}}"#);
        let recipes = BTreeMap::from([("swap".into(),swap.into_bytes()), ("normal".into(),normal.to_vec())]);
        let c = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        let allowed = |v: Option<&Value>, m: &str| material_predicate(v,m,&|_,_| false,&|_,_| None,&|_,_,_| None).unwrap_or(false);
        let got = c.part_swap_casting(true,"tconstruct:tool","test:metal",&|_,_| true,&allowed,&|_| true);
        assert_eq!(got.is_some(), expected, "{predicate}");
        if let Some((_, recipe, material, _)) = got { assert_eq!((material.as_str(), recipe.fluid.amount), ("tconstruct:iron", 180)); }
    }
}

}

#[cfg(test)]
mod slimelytra_tests {
    use super::*;
    #[test]
    fn consumed_tool_cast_selects_only_supported_unambiguous_material() {
        let recipe = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed","item_cost":8,"result":"tconstruct:slime_wings"}"#;
        let fluid = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"tag":"forge:slime"},"output":"tconstruct:earthslime","temperature":50}"#;
        let conditional = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed","item_cost":8,"result":"tconstruct:slime_wings","conditions":[{"type":"forge:false"}]}"#;
        assert!(super::super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(conditional, &|_| false).is_none());
        let unsupported = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed","item_cost":8,"result":"tconstruct:slime_wings","fluid_swapping":true}"#;
        assert!(super::super::tconstruct_source_melting_recipes::SlimelytraCastingRecipe::parse(unsupported, &|_| true).is_none());
        let recipes = [("test:tool".into(), recipe.to_vec()), ("test:fluid".into(), fluid.to_vec())].into_iter().collect();
        let item_tags = HashMap::new();
        let fluid_tags = [("forge:slime".into(), ["tconstruct:molten_slime".into()].into_iter().collect())].into_iter().collect();
        let known_items = ["minecraft:elytra".into(), "tconstruct:slime_wings".into()].into_iter().collect();
        let known_fluids = ["tconstruct:molten_slime".into()].into_iter().collect();
        let c = Catalog { recipes: &recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        let (_, r, material) = c.slimelytra_casting(true, "minecraft:elytra", "tconstruct:molten_slime", &|id| id == "tconstruct:earthslime", &|_| true).unwrap();
        assert_eq!(material, "tconstruct:earthslime");
        assert_eq!(r.fluid.amount, 2000); assert!(r.consumes_cast); assert!(!r.switch_slots); assert!(r.cooling_time > 0);
        assert!(c.slimelytra_casting(false, "minecraft:elytra", "tconstruct:molten_slime", &|_| true, &|_| true).is_none());
        assert!(c.slimelytra_casting(true, "minecraft:stone", "tconstruct:molten_slime", &|_| true, &|_| true).is_none());
        assert!(c.slimelytra_casting(true, "minecraft:elytra", "tconstruct:molten_slime", &|_| false, &|_| true).is_none());
        assert!(c.slimelytra_casting(true, "minecraft:elytra", "minecraft:water", &|_| true, &|_| true).is_none());
        assert!(c.slimelytra_casting(true, "minecraft:elytra", "tconstruct:molten_slime", &|_| true, &|_| false).is_some()); // no conditions to reject
        let mut ambiguous_recipes = recipes.clone();
        ambiguous_recipes.insert("test:other".into(), br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"tag":"forge:slime"},"output":"tconstruct:skyslime","temperature":50}"#.to_vec());
        let ambiguous = Catalog { recipes: &ambiguous_recipes, item_tags: &item_tags, fluid_tags: &fluid_tags, known_items: &known_items, known_fluids: &known_fluids };
        assert!(ambiguous.slimelytra_casting(true, "minecraft:elytra", "tconstruct:molten_slime", &|_| true, &|_| true).is_none());
    }
}

#[cfg(test)]
mod tipped_clearing_tests {
    use super::*;
    #[test]
    fn source_table_recipes_select_tool_tag_water_and_quantity() {
        let mut recipes = BTreeMap::new();
        recipes.insert("ammo".to_owned(), br#"{"type":"tconstruct:casting_table_tipped_clearing","cooling_time":20,"fluid":{"amount":50,"tag":"mantle:water"},"modifier":"tconstruct:tipped","tools":{"tag":"tconstruct:modifiable/ammo"}}"#.to_vec());
        recipes.insert("rod".to_owned(), br#"{"type":"tconstruct:casting_table_tipped_clearing","cooling_time":20,"fluid":{"amount":250,"tag":"mantle:water"},"modifier":"tconstruct:tipped","tools":{"tag":"tconstruct:modifiable/fishing_rods"}}"#.to_vec());
        let tags = HashMap::from([
            ("tconstruct:modifiable/ammo".into(), HashSet::from(["tconstruct:arrow".into()])),
            ("tconstruct:modifiable/fishing_rods".into(), HashSet::from(["tconstruct:fishing_rod".into()])),
        ]);
        let fluids = HashMap::from([("mantle:water".into(), HashSet::from(["minecraft:water".into()]))]);
        let items = HashSet::from(["tconstruct:arrow".into(), "tconstruct:fishing_rod".into()]);
        let known = HashSet::from(["minecraft:water".into(), "minecraft:lava".into()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &fluids, known_items: &items, known_fluids: &known };
        for (item, expected) in [("tconstruct:arrow", 50), ("tconstruct:fishing_rod", 250)] {
            let (_, recipe) = catalog.tip_clearing(false, item, "minecraft:water", &|_| true).unwrap();
            assert_eq!(recipe.fluid.amount, expected); assert_eq!(recipe.cooling_time, 20);
            assert!(recipe.consumes_cast); assert!(!recipe.switch_slots);
            assert!(catalog.tip_clearing(true, item, "minecraft:water", &|_| true).is_none());
            assert!(catalog.tip_clearing(false, item, "minecraft:lava", &|_| true).is_none());
        }
        assert!(catalog.tip_clearing(false, "minecraft:stone", "minecraft:water", &|_| true).is_none());
    }
    #[test]
    fn unsupported_difference_with_missing_tag_never_matches() {
        let recipes = BTreeMap::from([("unsafe".to_owned(), br#"{"type":"tconstruct:casting_table_tipped_clearing","fluid":{"amount":50,"fluid":"minecraft:water"},"modifier":"tconstruct:tipped","tools":{"type":"forge:difference","base":{"item":"tconstruct:arrow"},"subtracted":{"tag":"example:missing"}}}"#.to_vec())]);
        let tags = HashMap::new(); let items = HashSet::from(["tconstruct:arrow".to_owned()]);
        let fluids = HashSet::from(["minecraft:water".to_owned()]);
        let catalog = Catalog { recipes: &recipes, item_tags: &tags, fluid_tags: &tags, known_items: &items, known_fluids: &fluids };
        assert!(catalog.tip_clearing(false, "tconstruct:arrow", "minecraft:water", &|_| true).is_none());
    }
    #[test]
    fn unknown_modifier_and_false_conditions_fail_closed() {
        let invalid = br#"{"type":"tconstruct:casting_table_tipped_clearing","fluid":{"amount":50,"fluid":"minecraft:water"},"modifier":"example:unknown","tools":{"item":"tconstruct:arrow"}}"#;
        assert!(super::super::tconstruct_source_melting_recipes::TipClearingRecipe::parse(invalid, &|_| true).is_none());
        let gated = br#"{"type":"tconstruct:casting_table_tipped_clearing","fluid":{"amount":50,"fluid":"minecraft:water"},"modifier":"tconstruct:tipped","tools":{"item":"tconstruct:arrow"},"conditions":[{"type":"example:test"}]}"#;
        assert!(super::super::tconstruct_source_melting_recipes::TipClearingRecipe::parse(gated, &|_| false).is_none());
    }
}

#[cfg(test)]
mod first_material_tests {
    use super::*;
    #[test]
    fn fixed_forms_preserve_part_then_fluid_and_cost() {
        for (basin, part, result, cost, stat) in [(false,"shield_core","plate_shield",3,"plating_shield"), (true,"laces","slime_boots",4,"slime"), (true,"ribcage","slimy_chestplate",8,"slime"), (true,"shell","slime_leggings",7,"slime")] {
            let ty = if basin { "basin_tool_casting" } else { "table_tool_casting" };
            let raw = format!(r#"{{"type":"tconstruct:{ty}","cast":{{"type":"tconstruct:material","item":"tconstruct:{part}"}},"cast_purpose":"first_material","item_cost":{cost},"result":"tconstruct:{result}"}}"#);
            let fluid = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"fluid":"test:fluid"},"output":"test:second","temperature":50}"#;
            let recipes = BTreeMap::from([("tool".into(), raw.into_bytes()), ("fluid".into(), fluid.to_vec())]);
            let tags = HashMap::new();
            let items = HashSet::from([format!("tconstruct:{part}"), format!("tconstruct:{result}")]);
            let fluids = HashSet::from(["test:fluid".into()]);
            let c = Catalog { recipes:&recipes, item_tags:&tags, fluid_tags:&tags, known_items:&items, known_fluids:&fluids };
            let supports = |id: &str, kind: &str| (id == "test:first" && kind == format!("tconstruct:{part}")) || (id == "test:second" && kind == format!("tconstruct:{stat}"));
            let (_, r, out, second) = c.first_material_casting(basin, &format!("tconstruct:{part}"), "test:first", "test:fluid", &supports, &|_| true).unwrap();
            assert_eq!((out.as_str(), second.as_str(), r.fluid.amount, r.consumes_cast), (format!("tconstruct:{result}").as_str(), "test:second", 250 * cost, true));
            assert!(c.first_material_casting(!basin, &format!("tconstruct:{part}"), "test:first", "test:fluid", &supports, &|_| true).is_none());
            assert!(c.first_material_casting(basin, &format!("tconstruct:{part}"), "test:first", "test:fluid", &|_,_| false, &|_| true).is_none());
        }
    }
    #[test]
    fn unsupported_fields_and_cast_predicates_are_rejected() {
        use super::super::tconstruct_source_melting_recipes::FirstMaterialCastingRecipe as Recipe;
        let valid = r#"{"type":"tconstruct:basin_tool_casting","cast":{"type":"tconstruct:material","item":"tconstruct:laces"},"cast_purpose":"first_material","item_cost":4,"result":"tconstruct:slime_boots"}"#;
        assert!(Recipe::parse(valid.as_bytes(), &|_| true).is_some());
        for invalid in [valid.replace("\"item_cost\":4", "\"item_cost\":4,\"allowed_materials\":{}"), valid.replace("\"item\":\"tconstruct:laces\"", "\"item\":\"tconstruct:laces\",\"predicate\":{}"), valid.replace("slime_boots", "unknown")] {
            assert!(Recipe::parse(invalid.as_bytes(), &|_| true).is_none());
        }
    }
}

#[cfg(test)]
mod tool_fluid_swap_tests {
    use super::*;
    #[test]
    fn only_default_swapping_recipe_uses_new_last_material() {
        let boots = br#"{"type":"tconstruct:basin_tool_casting","cast":{"type":"tconstruct:material","item":"tconstruct:laces"},"cast_purpose":"first_material","item_cost":4,"result":"tconstruct:slime_boots"}"#;
        let skull = br#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:skeleton_skull"},"cast_purpose":"consumed_offset","extra_materials":["tconstruct:bone"],"fluid_swapping":false,"item_cost":5,"result":"tconstruct:slime_helmet"}"#;
        let fluid = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"fluid":"test:fluid"},"output":"test:new","temperature":50}"#;
        let recipes = BTreeMap::from([("boots".into(),boots.to_vec()),("skull".into(),skull.to_vec()),("fluid".into(),fluid.to_vec())]);
        let tags = HashMap::new(); let items = HashSet::from(["tconstruct:slime_boots".into(), "tconstruct:slime_helmet".into()]);
        let fluids = HashSet::from(["test:fluid".into()]);
        let c = Catalog { recipes:&recipes, item_tags:&tags, fluid_tags:&tags, known_items:&items, known_fluids:&fluids };
        let supports = |id: &str, stat: &str| id == "test:new" && stat == "tconstruct:slime";
        let (_, spec, result) = c.tool_fluid_swap(true,"tconstruct:slime_boots","test:old","test:fluid",&supports,&|_|true).unwrap();
        assert_eq!((result.as_str(),spec.fluid.amount,spec.consumes_cast), ("test:new",1000,true));
        assert!(c.tool_fluid_swap(true,"tconstruct:slime_boots","test:new#default","test:fluid",&supports,&|_|true).is_none());
        assert!(c.tool_fluid_swap(false,"tconstruct:slime_boots","test:old","test:fluid",&supports,&|_|true).is_none());
        assert!(c.tool_fluid_swap(true,"tconstruct:slime_helmet","test:old","test:fluid",&supports,&|_|true).is_none());
        assert!(c.tool_fluid_swap(true,"tconstruct:slime_boots","test:old","test:fluid",&|_,_|false,&|_|true).is_none());
    }
    #[test]
    fn wings_and_shield_follow_their_own_cost_and_stat() {
        for (basin, input, raw, stat, cost) in [
            (true, "tconstruct:slime_wings", r#"{"type":"tconstruct:basin_tool_casting","cast":{"item":"minecraft:elytra"},"cast_purpose":"consumed","item_cost":8,"result":"tconstruct:slime_wings"}"#, "tconstruct:slime", 8),
            (false, "tconstruct:plate_shield", r#"{"type":"tconstruct:table_tool_casting","cast":{"type":"tconstruct:material","item":"tconstruct:shield_core"},"cast_purpose":"first_material","item_cost":3,"result":"tconstruct:plate_shield"}"#, "tconstruct:plating_shield", 3),
        ] {
            let fluid = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"fluid":"test:fluid"},"output":"test:new","temperature":50}"#;
            let recipes = BTreeMap::from([("tool".into(), raw.as_bytes().to_vec()), ("fluid".into(), fluid.to_vec())]);
            let tags = HashMap::new(); let items = HashSet::from([input.into()]); let fluids = HashSet::from(["test:fluid".into()]);
            let c = Catalog { recipes:&recipes, item_tags:&tags, fluid_tags:&tags, known_items:&items, known_fluids:&fluids };
            let (_, recipe, _) = c.tool_fluid_swap(basin,input,"test:old","test:fluid",&|_, kind| kind == stat,&|_|true).unwrap();
            assert_eq!(recipe.fluid.amount, 250 * cost);
        }
    }
    #[test]
    fn explicit_part_swap_overlap_is_refused() {
        let boots = br#"{"type":"tconstruct:basin_tool_casting","cast":{"type":"tconstruct:material","item":"tconstruct:laces"},"cast_purpose":"first_material","item_cost":4,"result":"tconstruct:slime_boots"}"#;
        let other = br#"{"type":"tconstruct:basin_casting_part_swapping","tools":{"item":"tconstruct:slime_boots"},"item_cost":4,"index":1}"#;
        let recipes = BTreeMap::from([("boots".into(),boots.to_vec()),("other".into(),other.to_vec())]);
        let tags = HashMap::new(); let items = HashSet::from(["tconstruct:slime_boots".into()]); let fluids = HashSet::from(["test:fluid".into()]);
        let c = Catalog { recipes:&recipes, item_tags:&tags, fluid_tags:&tags, known_items:&items, known_fluids:&fluids };
        assert!(c.tool_swap_overlap(true, "tconstruct:slime_boots", &|_|true));
        assert!(!c.tool_swap_overlap(false, "tconstruct:slime_boots", &|_|true));
        assert!(!c.tool_swap_overlap(true, "tconstruct:other", &|_|true));
        assert!(c.tool_fluid_swap(true,"tconstruct:slime_boots","test:old","test:fluid",&|_,_|true,&|_|true).is_none());
        let reversed = BTreeMap::from([("a".into(),other.to_vec()),("z".into(),boots.to_vec())]);
        let c = Catalog { recipes:&reversed, item_tags:&tags, fluid_tags:&tags, known_items:&items, known_fluids:&fluids };
        assert!(c.tool_swap_overlap(true, "tconstruct:slime_boots", &|_|true));
        assert!(c.tool_fluid_swap(true,"tconstruct:slime_boots","test:old","test:fluid",&|_,_|true,&|_|true).is_none());
    }
}

#[cfg(test)]
mod slimeskull_part_swap_tests {
    use super::*;
    const SWAP: &[u8] = br#"{"type":"tconstruct:basin_casting_part_swapping","index":1,"item_cost":5,"tools":{"item":"tconstruct:slime_helmet"}}"#;
    const FLUID: &[u8] = br#"{"type":"tconstruct:material_fluid","fluid":{"amount":250,"fluid":"test:fluid"},"output":"test:new","temperature":50}"#;
    #[test]
    fn fixed_swap_uses_slime_stat_and_source_cost() {
        let recipes = BTreeMap::from([("swap".into(),SWAP.to_vec()),("fluid".into(),FLUID.to_vec())]);
        let tags=HashMap::new(); let items=HashSet::from(["tconstruct:slime_helmet".into()]); let fluids=HashSet::from(["test:fluid".into()]);
        let c=Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&tags,known_items:&items,known_fluids:&fluids};
        let support=|id: &str| id == "test:new";
        assert!(c.has_slimeskull_swap(true,"tconstruct:slime_helmet",&|_|true));
        let (_, recipe, result)=c.slimeskull_part_swap(true,"tconstruct:slime_helmet","test:old","test:fluid",&support,&|_|true).unwrap();
        assert_eq!((result.as_str(),recipe.fluid.amount,recipe.consumes_cast), ("test:new",1250,true));
        assert!(c.slimeskull_part_swap(true,"tconstruct:slime_helmet","test:new#default","test:fluid",&support,&|_|true).is_none());
        assert!(c.slimeskull_part_swap(false,"tconstruct:slime_helmet","test:old","test:fluid",&support,&|_|true).is_none());
        assert!(c.slimeskull_part_swap(true,"tconstruct:slime_helmet","test:old","test:fluid",&|_|false,&|_|true).is_none());
    }
    #[test]
    fn changed_semantics_and_competing_part_swap_fail_closed() {
        use super::super::tconstruct_source_melting_recipes::slimeskull_part_swap as parse;
        assert!(parse(SWAP,&|_|true));
        for change in [String::from_utf8(SWAP.to_vec()).unwrap().replace("\"index\":1","\"index\":0"), String::from_utf8(SWAP.to_vec()).unwrap().replace("\"item_cost\":5","\"item_cost\":5,\"allowed_materials\":{}"), String::from_utf8(SWAP.to_vec()).unwrap().replace("\"item_cost\":5","\"item_cost\":4")] {
            assert!(!parse(change.as_bytes(),&|_|true));
        }
        let other=br#"{"type":"tconstruct:basin_casting_part_swapping","index":0,"item_cost":5,"tools":{"item":"tconstruct:slime_helmet"}}"#;
        for recipes in [BTreeMap::from([("a".into(),SWAP.to_vec()),("z".into(),other.to_vec()),("fluid".into(),FLUID.to_vec())]),BTreeMap::from([("z".into(),SWAP.to_vec()),("a".into(),other.to_vec()),("fluid".into(),FLUID.to_vec())])] {
            let tags=HashMap::new(); let items=HashSet::from(["tconstruct:slime_helmet".into()]); let fluids=HashSet::from(["test:fluid".into()]);
            let c=Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&tags,known_items:&items,known_fluids:&fluids};
            assert!(c.has_slimeskull_swap(true,"tconstruct:slime_helmet",&|_|true));
            assert!(c.slimeskull_part_swap(true,"tconstruct:slime_helmet","test:old","test:fluid",&|_|true,&|_|true).is_none());
        }
    }
}
