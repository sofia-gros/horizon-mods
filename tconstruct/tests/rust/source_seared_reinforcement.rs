#[path = "../../../pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"]
mod pluto_stack_fluid;
#[path = "../../mixins/tconstruct/source-fluid-display.rs"]
mod tconstruct_source_fluid_display;
#[path = "../../mixins/tconstruct/source-melter-state.rs"]
mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-smeltery-tank.rs"]
mod tconstruct_source_smeltery_tank;
mod tconstruct_source_smeltery_fuel {
    pub(super) fn potion_fluid_name(tag: Option<&horizon_world::nbt::Tag>) -> Option<String> {
        match tag?.get("Potion")? {
            horizon_world::nbt::Tag::String(id)
                if [
                    "minecraft:swiftness",
                    "minecraft:long_swiftness",
                    "minecraft:strong_swiftness",
                ]
                .contains(&id.to_string_lossy().as_str()) =>
            {
                Some("item.minecraft.potion.effect.swiftness".into())
            }
            _ => None,
        }
    }
    pub(super) fn item_capacity(id: &str) -> Option<u32> {
        match id {
            "tconstruct:seared_fuel_tank"
            | "tconstruct:seared_fuel_gauge"
            | "tconstruct:scorched_fuel_tank"
            | "tconstruct:scorched_fuel_gauge"
            | "tconstruct:seared_casting_tank" => Some(4000),
            "tconstruct:seared_ingot_tank"
            | "tconstruct:seared_ingot_gauge"
            | "tconstruct:scorched_ingot_tank"
            | "tconstruct:scorched_ingot_gauge" => Some(4320),
            "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => Some(50),
            "tconstruct:seared_fluid_cannon" => Some(2000),
            _ => None,
        }
    }
}
#[path = "../../mixins/tconstruct/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[cfg(test)] mod tests {
 use super::*; use horizon_items::items::ItemStack; use std::collections::{BTreeMap,HashMap,HashSet};
 #[test] fn pinned_recipe_casts_with_each_fluid_and_consumes_pattern() {
  let bytes=include_bytes!("../fixtures/seared_reinforcement.json").to_vec();
  let recipes=BTreeMap::from([("tconstruct:tools/modifiers/seared_reinforcement".into(),bytes)]);
  let tags=HashMap::new(); let fluids=HashMap::from([
   ("tconstruct:seared_stone".into(),HashSet::from(["tconstruct:seared_stone".into()])),
   ("tconstruct:scorched_stone".into(),HashSet::from(["tconstruct:scorched_stone".into()])),
  ]);
  let known_items=HashSet::from(["tconstruct:pattern".into(),"tconstruct:seared_reinforcement".into()]);
  let known_fluids=HashSet::from(["tconstruct:seared_stone".into(),"tconstruct:scorched_stone".into(),"minecraft:water".into()]);
  let catalog=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&known_items,known_fluids:&known_fluids};
  for fluid in ["tconstruct:seared_stone","tconstruct:scorched_stone"] {
   let (id,recipe,output)=catalog.casting(false,Some("tconstruct:pattern"),fluid,&|_|true).unwrap();
   assert_eq!(recipe.fluid.amount,250);assert_eq!(recipe.cooling_time,89);assert!(recipe.consumes_cast);assert_eq!(output,"tconstruct:seared_reinforcement");
   let mut state=tconstruct_source_melter_state::Casting::new();state.input=Some(ItemStack::new("tconstruct:pattern",1));
   assert!(state.prepare(&id,recipe.fluid.amount,recipe.cooling_time,ItemStack::new(&output,1),recipe.consumes_cast,recipe.switch_slots));
   assert!(state.tank.insert(fluid,249));assert!(!state.tick(true));assert!(state.input.is_some());assert!(state.tank.insert(fluid,1));
   for _ in 0..88 {assert!(!state.tick(true));}assert!(state.tick(true));assert!(state.input.is_none());assert_eq!(state.tank.amount,0);assert_eq!(state.output.unwrap().item.as_ref(),output);
  }
  assert!(catalog.casting(false,Some("tconstruct:pattern"),"minecraft:water",&|_|true).is_none());
  assert!(catalog.casting(true,Some("tconstruct:pattern"),"tconstruct:seared_stone",&|_|true).is_none());
  assert!(catalog.casting(false,None,"tconstruct:seared_stone",&|_|true).is_none());
 }
 #[test] fn blazing_bone_source_recipe_requires_bone_blood_and_registered_output() {
  let bytes=include_bytes!("../fixtures/blazing_bone.json").to_vec();
  let recipes=BTreeMap::from([("tconstruct:smeltery/casting/blaze/bone".into(),bytes)]);
  let tags=HashMap::from([("forge:wither_bones".into(),HashSet::from(["example:wither_bone".into()]))]);
  let fluids=HashMap::from([("tconstruct:blazing_blood".into(),HashSet::from(["tconstruct:blazing_blood".into()]))]);
  let items=HashSet::from(["example:wither_bone".into(),"tconstruct:blazing_bone".into()]);
  let known_fluids=HashSet::from(["tconstruct:blazing_blood".into()]);
  let c=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&items,known_fluids:&known_fluids};
  let (_,r,out)=c.casting(false,Some("example:wither_bone"),"tconstruct:blazing_blood",&|_|true).unwrap();
  assert_eq!((r.fluid.amount,r.cooling_time,r.consumes_cast,out.as_str()),(200,120,true,"tconstruct:blazing_bone"));
  assert!(c.casting(true,Some("example:wither_bone"),"tconstruct:blazing_blood",&|_|true).is_none());
  assert!(c.casting(false,None,"tconstruct:blazing_blood",&|_|true).is_none());
 }
 #[test] fn silky_cloth_source_casting_and_melting_roundtrip() {
  let recipes=BTreeMap::from([
   ("tconstruct:tools/modifiers/silky_cloth".into(),include_bytes!("../fixtures/silky_cloth_casting.json").to_vec()),
   ("tconstruct:smeltery/melting/metal/rose_gold/silky_cloth".into(),include_bytes!("../fixtures/silky_cloth_melting.json").to_vec())]);
  let tags=HashMap::new();let fluids=HashMap::from([("forge:molten_rose_gold".into(),HashSet::from(["tconstruct:molten_rose_gold".into()]))]);
  let items=HashSet::from(["minecraft:cobweb".into(),"tconstruct:silky_cloth".into()]);let known_fluids=HashSet::from(["tconstruct:molten_rose_gold".into()]);
  let c=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&items,known_fluids:&known_fluids};
  let (_,r,out)=c.casting(false,Some("minecraft:cobweb"),"tconstruct:molten_rose_gold",&|_|true).unwrap();
  assert_eq!((r.fluid.amount,r.cooling_time,r.consumes_cast,out.as_str()),(90,52,true,"tconstruct:silky_cloth"));
  assert!(c.casting(true,Some("minecraft:cobweb"),"tconstruct:molten_rose_gold",&|_|true).is_none());
  let (_,r,out)=c.melting("tconstruct:silky_cloth",&|_|true).unwrap();
  assert_eq!((r.output.amount,r.temperature,r.time,out.as_str()),(90,550,52,"tconstruct:molten_rose_gold"));
 }
 #[test] fn invalid_and_different_amount_alternatives_fail_closed() {
  use tconstruct_source_melting_recipes::CastingRecipe;
  let recipe=include_str!("../fixtures/seared_reinforcement.json"); let mut value:serde_json::Value=serde_json::from_str(recipe).unwrap();
  value["fluid"][1]["amount"]=serde_json::json!(251);
  assert!(CastingRecipe::parse_for_fluid(&serde_json::to_vec(&value).unwrap(),&|_|true,&|_|true).is_none());
  value["fluid"][1]=serde_json::json!({"unknown":"bad","amount":250});
  assert!(CastingRecipe::parse_for_fluid(&serde_json::to_vec(&value).unwrap(),&|_|true,&|_|true).is_none());
 }
}
