#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;
#[test]
fn pinned_source_melting_and_casting_resolve_without_vanilla_recipe_changes() {
 use std::collections::{BTreeMap,HashMap,HashSet};use std::io::Read;
 let file=std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap();let mut zip=zip::ZipArchive::new(file).unwrap();
 let mut recipes=BTreeMap::new();
 for index in 0..zip.len() {
  let mut file=zip.by_index(index).unwrap();let name=file.name().to_owned();
  if !name.starts_with("data/tconstruct/recipes/") || !name.ends_with(".json") {continue;}
  let mut data=Vec::new();file.read_to_end(&mut data).unwrap();recipes.insert(name,data);
 }
 let items:HashSet<String>=["minecraft:obsidian","minecraft:emerald","tconstruct:obsidian_pane","tconstruct:emerald_reinforcement"].into_iter().map(str::to_owned).collect();
 let fluids:HashSet<String>=["tconstruct:molten_obsidian","tconstruct:molten_emerald"].into_iter().map(str::to_owned).collect();
 let item_tags:HashMap<String,HashSet<String>>=[("forge:obsidian",["minecraft:obsidian"]),("forge:gems/emerald",["minecraft:emerald"])].into_iter().map(|(k,v)|(k.to_owned(),v.into_iter().map(str::to_owned).collect())).collect();
 let fluid_tags:HashMap<String,HashSet<String>>=["tconstruct:molten_obsidian","tconstruct:molten_emerald"].into_iter().map(|v|(v.to_owned(),[v.to_owned()].into_iter().collect())).collect();
 let catalog=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&item_tags,fluid_tags:&fluid_tags,known_items:&items,known_fluids:&fluids};
 let (_,obsidian,fluid)=catalog.melting("minecraft:obsidian",&|_|true).unwrap();assert_eq!((fluid.as_str(),obsidian.output.amount,obsidian.temperature),("tconstruct:molten_obsidian",1000,1000));
 let (_,emerald,fluid)=catalog.melting("minecraft:emerald",&|_|true).unwrap();assert_eq!((fluid.as_str(),emerald.output.amount,emerald.temperature),("tconstruct:molten_emerald",100,934));
 let (_,pane,out)=catalog.casting(false,None,"tconstruct:molten_obsidian",&|_|true).unwrap();assert_eq!((out.as_str(),pane.fluid.amount,pane.consumes_cast),("tconstruct:obsidian_pane",250,false));
 let (_,plate,out)=catalog.casting(false,Some("tconstruct:obsidian_pane"),"tconstruct:molten_emerald",&|_|true).unwrap();assert_eq!((out.as_str(),plate.fluid.amount,plate.consumes_cast),("tconstruct:emerald_reinforcement",25,true));
}

#[test]
fn original_slimesteel_alloy_and_reinforcement_share_the_casting_catalog() {
 use std::collections::{BTreeMap,HashMap,HashSet}; use std::io::Read;
 let mut zip=zip::ZipArchive::new(std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap()).unwrap();
 let mut recipes=BTreeMap::new();
 for index in 0..zip.len() { let mut file=zip.by_index(index).unwrap(); let name=file.name().to_owned(); if name.starts_with("data/tconstruct/recipes/") && name.ends_with(".json") { let mut data=Vec::new();file.read_to_end(&mut data).unwrap();recipes.insert(name,data); } }
 let items:HashSet<String>=["tconstruct:sky_slime_crystal","tconstruct:obsidian_pane","tconstruct:slimesteel_reinforcement","minecraft:glass_bottle","tconstruct:sky_slime_bottle"].into_iter().map(str::to_owned).collect();
 let fluids:HashSet<String>=["tconstruct:sky_slime","tconstruct:seared_stone","tconstruct:molten_iron","tconstruct:molten_slimesteel"].into_iter().map(str::to_owned).collect();
 let tags:HashMap<String,HashSet<String>>=[("tconstruct:sky_slime","tconstruct:sky_slime"),("tconstruct:seared_stone","tconstruct:seared_stone"),("forge:molten_iron","tconstruct:molten_iron"),("tconstruct:molten_slimesteel","tconstruct:molten_slimesteel")].into_iter().map(|(tag,id)|(tag.into(),[id.into()].into_iter().collect())).collect();
 let item_tags=HashMap::new();let catalog=tconstruct_source_melting_lookup::Catalog{recipes:&recipes,item_tags:&item_tags,fluid_tags:&tags,known_items:&items,known_fluids:&fluids};
 let (_,crystal,fluid)=catalog.melting("tconstruct:sky_slime_crystal",&|_|true).unwrap();assert_eq!((fluid.as_str(),crystal.output.amount),("tconstruct:sky_slime",250));
 let (_,bottle,out)=catalog.casting(false,Some("minecraft:glass_bottle"),"tconstruct:sky_slime",&|_|true).unwrap();assert_eq!((out.as_str(),bottle.fluid.amount,bottle.consumes_cast),("tconstruct:sky_slime_bottle",250,true));
 let inputs=vec![("tconstruct:molten_iron".into(),90),("tconstruct:sky_slime".into(),250),("tconstruct:seared_stone".into(),250)];
 let (_,_,result,amount)=catalog.alloy(&inputs,1000,900,&|_|true).unwrap();assert_eq!((result.as_str(),amount),("tconstruct:molten_slimesteel",180));
 let (_,recipe,out)=catalog.casting(false,Some("tconstruct:obsidian_pane"),"tconstruct:molten_slimesteel",&|_|true).unwrap();assert_eq!((out.as_str(),recipe.fluid.amount,recipe.consumes_cast),("tconstruct:slimesteel_reinforcement",30,true));
}
