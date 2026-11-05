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
 #[test]
 fn named_head_cast_consumes_presentation_only_after_complete_pour() {
  use horizon_world::nbt::Tag;
  use tconstruct_source_melter_state::{Casting, consumed_head_with_presentation};
  let mut head = ItemStack::new("minecraft:skeleton_skull", 1);
  head.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Remember me".into())));
  head.data_mut().set_other("minecraft:lore", Some(Tag::List(vec![Tag::String("Named cast".into())])));
  assert!(consumed_head_with_presentation(&head));
  let mut state = Casting::new();
  state.input = Some(head.clone());
  assert!(state.prepare("tconstruct:tools/armor/slime_skull/bone", 1250, 2, ItemStack::new("tconstruct:slime_helmet", 1), true, false));
  assert!(state.tank.insert("tconstruct:earth_slime", 1249));
  assert!(!state.tick(true));
  assert_eq!(state.input, Some(head.clone()));
  state = Casting::from_nbt(&state.to_nbt()).unwrap();
  assert_eq!(state.input, Some(head));
  assert!(state.tank.insert("tconstruct:earth_slime", 1));
  state.tick(true); state.tick(true);
  assert!(state.input.is_none());
  let output = state.output.as_ref().unwrap();
  assert_eq!(output.item.as_ref(), "tconstruct:slime_helmet");
  assert!(output.data().is_none_or(|data| data.is_empty()));
 }
 #[test]
 fn named_head_cast_rejects_gameplay_components_and_invalid_presentation() {
  use horizon_world::nbt::Tag;
  use tconstruct_source_melter_state::consumed_head_with_presentation;
  let plain = ItemStack::new("minecraft:skeleton_skull", 1);
  assert!(consumed_head_with_presentation(&plain));
  for (component, value) in [
   ("minecraft:profile", Tag::compound([("name", Tag::String("Player".into()))])),
   ("minecraft:custom_data", Tag::compound([("extension", Tag::Int(1))])),
   ("minecraft:custom_name", Tag::Int(1)),
   ("minecraft:lore", Tag::List(vec![Tag::Int(1)])),
  ] {
   let mut head = plain.clone();
   if component == "minecraft:custom_name" {
    // Bypass the setter, which normalizes malformed names away.
    head.data_mut().custom_name = Some(value);
   } else { head.data_mut().set_other(component, Some(value)); }
   assert!(!consumed_head_with_presentation(&head), "{component}");
  }
  let mut wired = plain.clone();
  wired.data_mut().wire = horizon_items::item_data::Wire::new(plain.clone(), std::sync::Arc::from([0u8]));
  assert!(!consumed_head_with_presentation(&wired));
  wired.data_mut().set_other("minecraft:custom_name", Some(Tag::String("Changed".into())));
  assert!(wired.wire_patch().is_none()); // Retained wire remains unsafe even after a visible edit.
  assert!(!consumed_head_with_presentation(&wired));
  let mut stacked = plain.clone(); stacked.count = 2;
  assert!(!consumed_head_with_presentation(&stacked));
  let mut damaged = plain; damaged.damage = 1;
  assert!(!consumed_head_with_presentation(&damaged));
 }
 #[test] fn remaining_reinforcements_cast_only_with_registered_output_and_consume_pattern() {
  for (material,bytes,amount,ticks,tag) in [
   ("iron",include_bytes!("../fixtures/iron_reinforcement.json").as_slice(),90,60,"forge:molten_iron"),
   ("gold",include_bytes!("../fixtures/gold_reinforcement.json").as_slice(),90,57,"forge:molten_gold"),
   ("cobalt",include_bytes!("../fixtures/cobalt_reinforcement.json").as_slice(),90,65,"forge:molten_cobalt"),
   ("obsidian",include_bytes!("../fixtures/obsidian_reinforcement.json").as_slice(),1000,221,"tconstruct:molten_obsidian"),
  ] {
   let output=format!("tconstruct:{material}_reinforcement"); let fluid=format!("tconstruct:molten_{material}");
   let recipes=BTreeMap::from([(format!("tconstruct:tools/modifiers/{material}_reinforcement"),bytes.to_vec())]);
   let tags=HashMap::new(); let fluids=HashMap::from([(tag.into(),HashSet::from([fluid.clone()]))]);
   let items=HashSet::from(["tconstruct:pattern".into(),output.clone()]);let known_fluids=HashSet::from([fluid.clone(),"minecraft:water".into()]);
   let c=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&items,known_fluids:&known_fluids};
   let (id,r,out)=c.casting(false,Some("tconstruct:pattern"),&fluid,&|_|true).unwrap();
   assert_eq!((r.fluid.amount,r.cooling_time,r.consumes_cast,out.as_str()),(amount,ticks,true,output.as_str()));
   let missing_items=HashSet::from(["tconstruct:pattern".into()]);
   let missing=tconstruct_source_melting_lookup::Catalog{known_items:&missing_items,..c};
   assert!(missing.casting(false,Some("tconstruct:pattern"),&fluid,&|_|true).is_none());
   assert!(c.casting(true,Some("tconstruct:pattern"),&fluid,&|_|true).is_none());
   assert!(c.casting(false,None,&fluid,&|_|true).is_none());
   assert!(c.casting(false,Some("tconstruct:pattern"),"minecraft:water",&|_|true).is_none());
   let mut state=tconstruct_source_melter_state::Casting::new();state.input=Some(ItemStack::new("tconstruct:pattern",1));
   assert!(state.prepare(&id,amount,ticks,ItemStack::new(&out,1),r.consumes_cast,r.switch_slots));
   assert!(state.tank.insert(&fluid,amount-1));assert!(!state.tick(true));assert!(state.input.is_some());assert!(state.tank.insert(&fluid,1));
   for _ in 0..ticks-1 {assert!(!state.tick(true));}
   assert!(state.tick(true));assert!(state.input.is_none());assert_eq!(state.tank.amount,0);assert_eq!(state.output.unwrap().item.as_ref(),output);
  }
 }
 #[test] fn water_bottles_cast_with_potion_components_and_survive_state_roundtrip() {
  for (kind,input,output,bytes) in [
   ("bottle","minecraft:glass_bottle","minecraft:potion",include_bytes!("../fixtures/water_casting_bottle.json").as_slice()),
   ("splash","tconstruct:splash_bottle","minecraft:splash_potion",include_bytes!("../fixtures/water_casting_splash.json").as_slice()),
   ("lingering","tconstruct:lingering_bottle","minecraft:lingering_potion",include_bytes!("../fixtures/water_casting_lingering.json").as_slice()),
  ] {
   let recipes=BTreeMap::from([(format!("tconstruct:smeltery/casting/water/{kind}"),bytes.to_vec())]);
   let tags=HashMap::from([("forge:bottles/splash".into(),HashSet::from(["tconstruct:splash_bottle".into()])),("forge:bottles/lingering".into(),HashSet::from(["tconstruct:lingering_bottle".into()]))]);
   let fluids=HashMap::from([("mantle:water".into(),HashSet::from(["minecraft:water".into()]))]);
   let items=HashSet::from([input.into(),output.into()]);let known_fluids=HashSet::from(["minecraft:water".into(),"minecraft:lava".into()]);
   let c=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&items,known_fluids:&known_fluids};
   let (id,r,out)=c.casting(false,Some(input),"minecraft:water",&|_|true).unwrap();
   assert_eq!((r.fluid.amount,r.cooling_time,r.consumes_cast,out.as_str()),(500,1,true,output));
   assert_eq!(r.output.potion(),Some("minecraft:water"));
   assert!(c.casting(true,Some(input),"minecraft:water",&|_|true).is_none());
   assert!(c.casting(false,None,"minecraft:water",&|_|true).is_none());
   assert!(c.casting(false,Some(input),"minecraft:lava",&|_|true).is_none());
   let missing_items=HashSet::from([input.into()]);
   assert!(tconstruct_source_melting_lookup::Catalog{known_items:&missing_items,..c}.casting(false,Some(input),"minecraft:water",&|_|true).is_none());
   let empty_tags=HashMap::new();
   assert!(tconstruct_source_melting_lookup::Catalog{fluid_tags:&empty_tags,..c}.casting(false,Some(input),"minecraft:water",&|_|true).is_none());
   let result=ItemStack::new(&out,1).with_potion(r.output.potion().unwrap());
   let mut state=tconstruct_source_melter_state::Casting::new();state.input=Some(ItemStack::new(input,1));
   assert!(state.prepare(&id,500,1,result,true,r.switch_slots));
   assert!(state.tank.insert("minecraft:water",499));assert!(!state.tick(true));assert!(state.input.is_some());
   let mut state=tconstruct_source_melter_state::Casting::from_nbt(&state.to_nbt()).unwrap();
   assert!(state.tank.insert("minecraft:water",1));assert!(state.tick(true));assert!(state.input.is_none());assert_eq!(state.tank.amount,0);
   let state=tconstruct_source_melter_state::Casting::from_nbt(&state.to_nbt()).unwrap();
   let result=state.output.unwrap();assert_eq!(result.item.as_ref(),output);assert_eq!(result.potion(),Some("minecraft:water"));assert_eq!(result.count,1);
  }
 }
 #[test] fn water_result_parser_rejects_unknown_nbt_and_ambiguous_plain_potion() {
  use tconstruct_source_melting_recipes::CastingRecipe;
  let source=include_bytes!("../fixtures/water_casting_bottle.json");
  let original:serde_json::Value=serde_json::from_slice(source).unwrap();
  for result in [
   serde_json::json!({"item":"minecraft:potion","nbt":{"Potion":"minecraft:healing"}}),
   serde_json::json!({"item":"minecraft:potion","nbt":{"Potion":"minecraft:water","CustomPotionColor":1}}),
   serde_json::json!({"item":"minecraft:potion","nbt":{"Potion":"minecraft:water"},"count":2}),
   serde_json::json!({"item":"minecraft:stone","nbt":{"Potion":"minecraft:water"}}),
   serde_json::json!({"item":"minecraft:potion","nbt":"{Potion:water}"}),
  ] {let mut v=original.clone();v["result"]=result;assert!(CastingRecipe::parse(&serde_json::to_vec(&v).unwrap(),&|_|true).is_none());}
  let mut v=original.clone();v["conditions"]=serde_json::json!([{"type":"example:off"}]);assert!(CastingRecipe::parse(&serde_json::to_vec(&v).unwrap(),&|_|false).is_none());
  let mut plain=original.clone();plain["result"]=serde_json::json!("minecraft:potion");
  let recipes=BTreeMap::from([("test:water".into(),source.to_vec()),("test:plain".into(),serde_json::to_vec(&plain).unwrap())]);
  let tags=HashMap::new();let fluids=HashMap::from([("mantle:water".into(),HashSet::from(["minecraft:water".into()]))]);
  let items=HashSet::from(["minecraft:glass_bottle".into(),"minecraft:potion".into()]);let known_fluids=HashSet::from(["minecraft:water".into()]);
  let c=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&tags,fluid_tags:&fluids,known_items:&items,known_fluids:&known_fluids};
  assert!(c.casting(false,Some("minecraft:glass_bottle"),"minecraft:water",&|_|true).is_none());
 }
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
