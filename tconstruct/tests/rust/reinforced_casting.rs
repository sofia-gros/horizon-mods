#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/source-melting-recipes.rs"] mod tconstruct_source_melting_recipes;
#[path = "../../mixins/tconstruct/source-melting-lookup.rs"] mod tconstruct_source_melting_lookup;
#[test]
fn pinned_source_melting_and_casting_resolve_without_vanilla_recipe_changes() {
 use std::collections::{BTreeMap,HashMap,HashSet};use std::io::Read;
 let file=std::fs::File::open("tconstruct/content/reference/tconstruct-recipes.zip").unwrap();let mut zip=zip::ZipArchive::new(file).unwrap();
 let mut recipes=BTreeMap::new();
 for name in ["smeltery/melting/obsidian/block","smeltery/melting/emerald/gem","smeltery/casting/obsidian/pane","tools/modifiers/emerald_reinforcement"] {
  let mut data=Vec::new();zip.by_name(&format!("data/tconstruct/recipes/{name}.json")).unwrap().read_to_end(&mut data).unwrap();recipes.insert(name.to_string(),data);
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
