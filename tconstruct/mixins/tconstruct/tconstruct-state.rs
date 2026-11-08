// All of Tinkers' per-world state in one place. Mixins plans every [[state]] separately (about 7 s each), so the 108 fields live in
// one struct behind one state, and the old accessor names are kept as thin methods so no helper had to change.
#[derive(Default)]
pub struct State {
    pub part_selections: std::collections::HashMap<(i32,i32,i32),String>,
    pub recipe_tag_snapshot: Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>>,
    pub material_snapshot: Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>>,
    pub material_tag_snapshot: Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>>,
    pub resolved_item_tags: Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>>,
    pub resolved_block_ids: Option<Result<std::collections::HashMap<String,i32>,String>>,
    pub resolved_block_tags: Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>>,
    pub smeltery_owners: std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)>,
    pub smeltery_guest_pages: std::collections::HashMap<(u32,u64,(i32,i32,i32)),(usize,usize)>,
    pub smeltery_ui_pages: std::collections::HashMap<(i32,i32,i32),(usize,usize)>,
    pub smeltery_ui_dragging: std::collections::HashSet<(i32,i32,i32)>,
    pub smeltery_ui_menu_state: std::collections::HashMap<(i32,i32,i32),(i32,i32)>,
    pub smeltery_local_containers: std::collections::HashMap<(i32,i32,i32),([Option<horizon_items::items::ItemStack>;2],u8)>,
    pub smeltery_guest_containers: std::collections::HashMap<u32,((i32,i32,i32),[Option<horizon_items::items::ItemStack>;2],u8)>,
    pub fluid_tag_snapshot: Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>>,
    pub resolved_fluid_tags: Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>>,
    pub resolved_material_definitions: Option<Result<(std::collections::HashMap<String,(bool,bool,i32,i32,String)>,std::collections::HashMap<String,String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>>,
    pub resolved_material_stats: Option<Result<std::collections::HashMap<String,std::collections::HashMap<String,bool>>,String>>,
    pub render_assets: std::sync::OnceLock<Result<(std::collections::HashMap<String,serde_json::Value>,std::collections::HashMap<String,(String,[f32;2])>),String>>,
    pub render_sprite_frames: std::sync::Mutex<std::collections::HashMap<String,(usize,usize,Vec<Vec<u8>>)>>,
    pub source_config_snapshot: Option<Result<std::collections::HashMap<String,bool>,String>>,
    pub source_ore_rates: Option<Result<std::collections::HashMap<String,u32>,String>>,
    pub recipe_mod_snapshot: Option<(std::collections::HashMap<String,Option<bool>>,bool)>,
    pub recipe_snapshot: Option<std::collections::BTreeMap<String,Vec<u8>>>,
    pub station_building_recipes: Option<Vec<Vec<u8>>>,
    pub material_wire_sent: std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag>,
    pub material_wire_client: bool,
    pub material_wire_ids: Option<(i32, i32)>,
    pub material_wire_before: std::collections::HashMap<(u32, u64), std::collections::HashSet<(i32, i32)>>,
    pub builder_choices: std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<Vec<String>>)>,
    pub builder_info: std::collections::HashMap<(i32,i32,i32),Vec<String>>,
    pub builder_info_scroll: std::collections::HashMap<(i32,i32,i32),usize>,
    pub station_info_scroll: std::collections::HashMap<(i32,i32,i32),(usize,usize)>,
    pub builder_side_scroll: std::collections::HashMap<(i32,i32,i32),(usize,bool)>,
    pub builder_side_title: std::collections::HashMap<(i32,i32,i32),i16>,
    pub builder_info_recv: std::collections::HashMap<(i32,i32,i32),Vec<i16>>,
    pub builder_info_sent: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>>,
    pub station_name_edit: std::collections::HashMap<(i32,i32,i32),crate::ui::EditBox>,
    pub station_guest_names: std::collections::HashMap<(u32,i32,(i32,i32,i32)),String>,
    pub builder_wire_recv: std::collections::HashMap<(i32,i32,i32),Vec<i16>>,
    pub builder_preview: std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<horizon_items::items::ItemStack>)>,
    pub source_native_durability: Option<std::collections::HashMap<String,u32>>,
    pub source_cast_costs: Option<Vec<i32>>,
    pub builder_cast_costs: std::collections::HashMap<(i32,i32,i32),Vec<i32>>,
    pub builder_cast_wire_recv: std::collections::HashMap<(i32,i32,i32),Vec<i16>>,
    pub builder_cast_wire_sent: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>>,
    pub builder_scroll: std::collections::HashMap<(i32,i32,i32),(f32,usize,bool,Vec<String>)>,
    pub builder_preview_sent: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>)>,
    pub source_recipe_generation: u64,
    pub builder_recipe_context: std::collections::HashMap<(i32,i32,i32),(u64,std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>,Vec<(String,String)>)>,
    pub source_shared_recipes: Option<std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>>,
    pub builder_material_cache: std::collections::HashMap<(i32,i32,i32),(horizon_items::items::ItemStack,Vec<u8>)>,
    pub source_selection_origin: std::collections::HashMap<(i32,i32,i32),(String,String,u64)>,
    pub builder_side: std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)>,
    pub builder_side_size: std::collections::HashMap<(i32,i32,i32),usize>,
    pub builder_double_partner: std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)>,
    pub station_preview_sent: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<String>)>,
    pub builder_crafted: Vec<(u32,String,u32,[f64;3])>,
    pub tool_models: std::sync::OnceLock<Result<std::collections::HashMap<String,Vec<(String,Option<usize>)>>,String>>,
    pub tank_fluid_context: Option<Result<(std::collections::HashSet<String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>>,
    pub melter_known_items: Option<std::collections::HashSet<String>>,
    pub machine_wire_sent: std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag>,
    pub remote_screen_position: Option<(i32,i32,i32)>,
    pub station_guest_layout_count: std::collections::HashMap<(u32,i32,(i32,i32,i32)),usize>,
    pub station_armor_preview: std::collections::HashMap<(i32,i32,i32),(f32,Option<f32>)>,
    pub area_guest_hits: std::collections::HashMap<u32,([i32;3],[i32;3],std::collections::HashSet<(i32,i32)>)>,
    pub area_active: std::collections::HashSet<Option<u32>>,
    pub area_overwield_spent: std::collections::HashSet<Option<u32>>,
    pub transform_guest_hits: std::collections::HashMap<u32,([i32;3],std::collections::HashSet<(i32,i32)>)>,
    pub melee_context: std::collections::HashMap<Option<u32>,(u32,u32,f32)>,
    pub melee_active: std::collections::HashSet<Option<u32>>,
    pub material_projectile_context: std::collections::HashMap<Option<u32>,(f32,f32,Vec<(f64,f32,bool)>,bool,f64,i32,bool)>,
    pub armor_wear_context: std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>>,
    pub glow_left_held: bool,
    pub glow_guest_protection: std::collections::HashMap<u32,Option<([i32;3],u32)>>,
    pub glow_support_checks: std::collections::HashSet<[i32; 3]>,
    pub modifier_wear_cause: std::collections::HashMap<Option<u32>, String>,
    pub overslime_pending: std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>>,
    pub materials_book_history: Option<usize>,
    pub materials_book_request_sequence: u64,
    pub chest_guest_pages: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(usize,usize)>,
    pub chest_ui_pages: std::collections::HashMap<(i32,i32,i32),(usize,usize)>,
    pub crafting_sessions: std::collections::HashSet<(i32,i32,i32)>,
    pub crafting_guest_sessions: std::collections::HashMap<(u32,u64,i32),(i32,i32,i32)>,
    pub crafting_guest_side: std::collections::HashMap<(u32,u64,i32),((i32,i32,i32),usize)>,
    pub crafting_side_client: Option<(i32,usize,usize,Vec<Option<horizon_items::items::ItemStack>>)>,
    pub menu_window_state: Option<(i32,i32)>,
    pub crafting_guest_views: std::collections::HashMap<(u32,u64,i32),(Vec<Option<horizon_items::items::ItemStack>>,Vec<Option<horizon_items::items::ItemStack>>)>,
    pub chest_guest_drag: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,Vec<(bool,usize)>)>,
    pub crafting_guest_drag: std::collections::HashMap<(u32,u64,i32),(u8,Vec<(bool,usize)>)>,
    pub chest_guest_last_move: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,horizon_items::items::ItemStack)>,
    pub crafting_guest_last_move: std::collections::HashMap<(u32,u64,i32),(u8,horizon_items::items::ItemStack)>,
    pub menu_wire_sent: std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i16,Vec<i16>)>,
    pub necrotic_context: std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>>,
    pub necrotic_projectile: std::collections::HashMap<Option<u32>, (u32, f32)>,
    pub necrotic_pvp: Vec<Option<(u32, Option<u32>, [i32; 4], Option<f32>, bool)>>,
    pub necrotic_host_victim: std::collections::HashMap<u32, (u32, [i32; 4], f32)>,
    pub spilling_context: std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>>,
    pub splashing_guest_scope: std::collections::HashMap<u32, Vec<(Option<u32>, i32, [f64; 3], [f64; 3], [f64; 3], bool)>>,
    pub splashing_guest_pending: std::collections::HashMap<u32, (horizon_items::items::ItemStack, usize, Vec<u32>, Vec<u32>, bool, i32, bool)>,
    pub splashing_guest_packet: std::collections::HashMap<u32, (i32, bool)>,
    pub haste_wire_client: Option<(horizon_items::items::ItemStack,i32)>,
    pub haste_wire_sent: std::collections::HashMap<(u32,u64),(u64,bool,Option<horizon_items::items::ItemStack>,(Vec<u8>,u8))>,
    pub broad_wire_client: Option<std::collections::HashSet<String>>,
    pub broad_wire_sent: std::collections::HashMap<(u32,u64),(Vec<u8>,u8)>,
    pub modifier_resource_snapshot: Option<Result<(std::collections::BTreeMap<String,Vec<Vec<u8>>>,std::collections::BTreeMap<String,Vec<Vec<u8>>>),String>>,
    pub lan_craft_recipe_generation: u64,
    pub lan_craft_menu_generation: std::collections::HashMap<(u32, u64, i32), u64>,
}

impl super::ChunkMap {
    pub fn __horizon_mixins_tconstruct_part_selections(&self) -> &std::collections::HashMap<(i32,i32,i32),String> { &self.__horizon_mixins_tconstruct_all_state().part_selections }
    pub fn __horizon_mixins_tconstruct_part_selections_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),String> { &mut self.__horizon_mixins_tconstruct_all_state_mut().part_selections }
    pub fn __horizon_mixins_tconstruct_recipe_tag_snapshot(&self) -> &Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &self.__horizon_mixins_tconstruct_all_state().recipe_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_recipe_tag_snapshot_mut(&mut self) -> &mut Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().recipe_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_material_snapshot(&self) -> &Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &self.__horizon_mixins_tconstruct_all_state().material_snapshot }
    pub fn __horizon_mixins_tconstruct_material_snapshot_mut(&mut self) -> &mut Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_snapshot }
    pub fn __horizon_mixins_tconstruct_material_tag_snapshot(&self) -> &Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &self.__horizon_mixins_tconstruct_all_state().material_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_material_tag_snapshot_mut(&mut self) -> &mut Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self) -> &Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_item_tags }
    pub fn __horizon_mixins_tconstruct_resolved_item_tags_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_item_tags }
    pub fn __horizon_mixins_tconstruct_resolved_block_ids(&self) -> &Option<Result<std::collections::HashMap<String,i32>,String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_block_ids }
    pub fn __horizon_mixins_tconstruct_resolved_block_ids_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,i32>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_block_ids }
    pub fn __horizon_mixins_tconstruct_resolved_block_tags(&self) -> &Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_block_tags }
    pub fn __horizon_mixins_tconstruct_resolved_block_tags_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_block_tags }
    pub fn __horizon_mixins_tconstruct_smeltery_owners(&self) -> &std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_owners }
    pub fn __horizon_mixins_tconstruct_smeltery_owners_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_owners }
    pub fn __horizon_mixins_tconstruct_smeltery_guest_pages(&self) -> &std::collections::HashMap<(u32,u64,(i32,i32,i32)),(usize,usize)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_guest_pages }
    pub fn __horizon_mixins_tconstruct_smeltery_guest_pages_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,(i32,i32,i32)),(usize,usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_guest_pages }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_pages(&self) -> &std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_ui_pages }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_pages_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_ui_pages }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_dragging(&self) -> &std::collections::HashSet<(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_ui_dragging }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_dragging_mut(&mut self) -> &mut std::collections::HashSet<(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_ui_dragging }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_menu_state(&self) -> &std::collections::HashMap<(i32,i32,i32),(i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_ui_menu_state }
    pub fn __horizon_mixins_tconstruct_smeltery_ui_menu_state_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_ui_menu_state }
    pub fn __horizon_mixins_tconstruct_smeltery_local_containers(&self) -> &std::collections::HashMap<(i32,i32,i32),([Option<horizon_items::items::ItemStack>;2],u8)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_local_containers }
    pub fn __horizon_mixins_tconstruct_smeltery_local_containers_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),([Option<horizon_items::items::ItemStack>;2],u8)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_local_containers }
    pub fn __horizon_mixins_tconstruct_smeltery_guest_containers(&self) -> &std::collections::HashMap<u32,((i32,i32,i32),[Option<horizon_items::items::ItemStack>;2],u8)> { &self.__horizon_mixins_tconstruct_all_state().smeltery_guest_containers }
    pub fn __horizon_mixins_tconstruct_smeltery_guest_containers_mut(&mut self) -> &mut std::collections::HashMap<u32,((i32,i32,i32),[Option<horizon_items::items::ItemStack>;2],u8)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().smeltery_guest_containers }
    pub fn __horizon_mixins_tconstruct_fluid_tag_snapshot(&self) -> &Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &self.__horizon_mixins_tconstruct_all_state().fluid_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_fluid_tag_snapshot_mut(&mut self) -> &mut Option<std::collections::BTreeMap<String,Vec<Vec<u8>>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().fluid_tag_snapshot }
    pub fn __horizon_mixins_tconstruct_resolved_fluid_tags(&self) -> &Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_fluid_tags }
    pub fn __horizon_mixins_tconstruct_resolved_fluid_tags_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,std::collections::HashSet<String>>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_fluid_tags }
    pub fn __horizon_mixins_tconstruct_resolved_material_definitions(&self) -> &Option<Result<(std::collections::HashMap<String,(bool,bool,i32,i32,String)>,std::collections::HashMap<String,String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_material_definitions }
    pub fn __horizon_mixins_tconstruct_resolved_material_definitions_mut(&mut self) -> &mut Option<Result<(std::collections::HashMap<String,(bool,bool,i32,i32,String)>,std::collections::HashMap<String,String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_material_definitions }
    pub fn __horizon_mixins_tconstruct_resolved_material_stats(&self) -> &Option<Result<std::collections::HashMap<String,std::collections::HashMap<String,bool>>,String>> { &self.__horizon_mixins_tconstruct_all_state().resolved_material_stats }
    pub fn __horizon_mixins_tconstruct_resolved_material_stats_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,std::collections::HashMap<String,bool>>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().resolved_material_stats }
    pub fn __horizon_mixins_tconstruct_render_assets(&self) -> &std::sync::OnceLock<Result<(std::collections::HashMap<String,serde_json::Value>,std::collections::HashMap<String,(String,[f32;2])>),String>> { &self.__horizon_mixins_tconstruct_all_state().render_assets }
    pub fn __horizon_mixins_tconstruct_render_assets_mut(&mut self) -> &mut std::sync::OnceLock<Result<(std::collections::HashMap<String,serde_json::Value>,std::collections::HashMap<String,(String,[f32;2])>),String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().render_assets }
    pub fn __horizon_mixins_tconstruct_render_sprite_frames(&self) -> &std::sync::Mutex<std::collections::HashMap<String,(usize,usize,Vec<Vec<u8>>)>> { &self.__horizon_mixins_tconstruct_all_state().render_sprite_frames }
    pub fn __horizon_mixins_tconstruct_render_sprite_frames_mut(&mut self) -> &mut std::sync::Mutex<std::collections::HashMap<String,(usize,usize,Vec<Vec<u8>>)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().render_sprite_frames }
    pub fn __horizon_mixins_tconstruct_source_config_snapshot(&self) -> &Option<Result<std::collections::HashMap<String,bool>,String>> { &self.__horizon_mixins_tconstruct_all_state().source_config_snapshot }
    pub fn __horizon_mixins_tconstruct_source_config_snapshot_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,bool>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_config_snapshot }
    pub fn __horizon_mixins_tconstruct_source_ore_rates(&self) -> &Option<Result<std::collections::HashMap<String,u32>,String>> { &self.__horizon_mixins_tconstruct_all_state().source_ore_rates }
    pub fn __horizon_mixins_tconstruct_source_ore_rates_mut(&mut self) -> &mut Option<Result<std::collections::HashMap<String,u32>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_ore_rates }
    pub fn __horizon_mixins_tconstruct_recipe_mod_snapshot(&self) -> &Option<(std::collections::HashMap<String,Option<bool>>,bool)> { &self.__horizon_mixins_tconstruct_all_state().recipe_mod_snapshot }
    pub fn __horizon_mixins_tconstruct_recipe_mod_snapshot_mut(&mut self) -> &mut Option<(std::collections::HashMap<String,Option<bool>>,bool)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().recipe_mod_snapshot }
    pub fn __horizon_mixins_tconstruct_recipe_snapshot(&self) -> &Option<std::collections::BTreeMap<String,Vec<u8>>> { &self.__horizon_mixins_tconstruct_all_state().recipe_snapshot }
    pub fn __horizon_mixins_tconstruct_recipe_snapshot_mut(&mut self) -> &mut Option<std::collections::BTreeMap<String,Vec<u8>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().recipe_snapshot }
    pub fn __horizon_mixins_tconstruct_station_building_recipes(&self) -> &Option<Vec<Vec<u8>>> { &self.__horizon_mixins_tconstruct_all_state().station_building_recipes }
    pub fn __horizon_mixins_tconstruct_station_building_recipes_mut(&mut self) -> &mut Option<Vec<Vec<u8>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_building_recipes }
    pub fn __horizon_mixins_tconstruct_material_wire_sent(&self) -> &std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag> { &self.__horizon_mixins_tconstruct_all_state().material_wire_sent }
    pub fn __horizon_mixins_tconstruct_material_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_wire_sent }
    pub fn __horizon_mixins_tconstruct_material_wire_client(&self) -> &bool { &self.__horizon_mixins_tconstruct_all_state().material_wire_client }
    pub fn __horizon_mixins_tconstruct_material_wire_client_mut(&mut self) -> &mut bool { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_wire_client }
    pub fn __horizon_mixins_tconstruct_material_wire_ids(&self) -> &Option<(i32, i32)> { &self.__horizon_mixins_tconstruct_all_state().material_wire_ids }
    pub fn __horizon_mixins_tconstruct_material_wire_ids_mut(&mut self) -> &mut Option<(i32, i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_wire_ids }
    pub fn __horizon_mixins_tconstruct_material_wire_before(&self) -> &std::collections::HashMap<(u32, u64), std::collections::HashSet<(i32, i32)>> { &self.__horizon_mixins_tconstruct_all_state().material_wire_before }
    pub fn __horizon_mixins_tconstruct_material_wire_before_mut(&mut self) -> &mut std::collections::HashMap<(u32, u64), std::collections::HashSet<(i32, i32)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_wire_before }
    pub fn __horizon_mixins_tconstruct_builder_choices(&self) -> &std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<Vec<String>>)> { &self.__horizon_mixins_tconstruct_all_state().builder_choices }
    pub fn __horizon_mixins_tconstruct_builder_choices_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<Vec<String>>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_choices }
    pub fn __horizon_mixins_tconstruct_builder_info(&self) -> &std::collections::HashMap<(i32,i32,i32),Vec<String>> { &self.__horizon_mixins_tconstruct_all_state().builder_info }
    pub fn __horizon_mixins_tconstruct_builder_info_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),Vec<String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_info }
    pub fn __horizon_mixins_tconstruct_builder_info_scroll(&self) -> &std::collections::HashMap<(i32,i32,i32),usize> { &self.__horizon_mixins_tconstruct_all_state().builder_info_scroll }
    pub fn __horizon_mixins_tconstruct_builder_info_scroll_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),usize> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_info_scroll }
    pub fn __horizon_mixins_tconstruct_station_info_scroll(&self) -> &std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &self.__horizon_mixins_tconstruct_all_state().station_info_scroll }
    pub fn __horizon_mixins_tconstruct_station_info_scroll_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_info_scroll }
    pub fn __horizon_mixins_tconstruct_builder_side_scroll(&self) -> &std::collections::HashMap<(i32,i32,i32),(usize,bool)> { &self.__horizon_mixins_tconstruct_all_state().builder_side_scroll }
    pub fn __horizon_mixins_tconstruct_builder_side_scroll_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(usize,bool)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_side_scroll }
    pub fn __horizon_mixins_tconstruct_builder_side_title(&self) -> &std::collections::HashMap<(i32,i32,i32),i16> { &self.__horizon_mixins_tconstruct_all_state().builder_side_title }
    pub fn __horizon_mixins_tconstruct_builder_side_title_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),i16> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_side_title }
    pub fn __horizon_mixins_tconstruct_builder_info_recv(&self) -> &std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &self.__horizon_mixins_tconstruct_all_state().builder_info_recv }
    pub fn __horizon_mixins_tconstruct_builder_info_recv_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_info_recv }
    pub fn __horizon_mixins_tconstruct_builder_info_sent(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>> { &self.__horizon_mixins_tconstruct_all_state().builder_info_sent }
    pub fn __horizon_mixins_tconstruct_builder_info_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_info_sent }
    pub fn __horizon_mixins_tconstruct_station_name_edit(&self) -> &std::collections::HashMap<(i32,i32,i32),crate::ui::EditBox> { &self.__horizon_mixins_tconstruct_all_state().station_name_edit }
    pub fn __horizon_mixins_tconstruct_station_name_edit_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),crate::ui::EditBox> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_name_edit }
    pub fn __horizon_mixins_tconstruct_station_guest_names(&self) -> &std::collections::HashMap<(u32,i32,(i32,i32,i32)),String> { &self.__horizon_mixins_tconstruct_all_state().station_guest_names }
    pub fn __horizon_mixins_tconstruct_station_guest_names_mut(&mut self) -> &mut std::collections::HashMap<(u32,i32,(i32,i32,i32)),String> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_guest_names }
    pub fn __horizon_mixins_tconstruct_builder_wire_recv(&self) -> &std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &self.__horizon_mixins_tconstruct_all_state().builder_wire_recv }
    pub fn __horizon_mixins_tconstruct_builder_wire_recv_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_wire_recv }
    pub fn __horizon_mixins_tconstruct_builder_preview(&self) -> &std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<horizon_items::items::ItemStack>)> { &self.__horizon_mixins_tconstruct_all_state().builder_preview }
    pub fn __horizon_mixins_tconstruct_builder_preview_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<horizon_items::items::ItemStack>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_preview }
    pub fn __horizon_mixins_tconstruct_source_native_durability(&self) -> &Option<std::collections::HashMap<String,u32>> { &self.__horizon_mixins_tconstruct_all_state().source_native_durability }
    pub fn __horizon_mixins_tconstruct_source_native_durability_mut(&mut self) -> &mut Option<std::collections::HashMap<String,u32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_native_durability }
    pub fn __horizon_mixins_tconstruct_source_cast_costs(&self) -> &Option<Vec<i32>> { &self.__horizon_mixins_tconstruct_all_state().source_cast_costs }
    pub fn __horizon_mixins_tconstruct_source_cast_costs_mut(&mut self) -> &mut Option<Vec<i32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_cast_costs }
    pub fn __horizon_mixins_tconstruct_builder_cast_costs(&self) -> &std::collections::HashMap<(i32,i32,i32),Vec<i32>> { &self.__horizon_mixins_tconstruct_all_state().builder_cast_costs }
    pub fn __horizon_mixins_tconstruct_builder_cast_costs_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),Vec<i32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_cast_costs }
    pub fn __horizon_mixins_tconstruct_builder_cast_wire_recv(&self) -> &std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &self.__horizon_mixins_tconstruct_all_state().builder_cast_wire_recv }
    pub fn __horizon_mixins_tconstruct_builder_cast_wire_recv_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),Vec<i16>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_cast_wire_recv }
    pub fn __horizon_mixins_tconstruct_builder_cast_wire_sent(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>> { &self.__horizon_mixins_tconstruct_all_state().builder_cast_wire_sent }
    pub fn __horizon_mixins_tconstruct_builder_cast_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),Vec<i16>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_cast_wire_sent }
    pub fn __horizon_mixins_tconstruct_builder_scroll(&self) -> &std::collections::HashMap<(i32,i32,i32),(f32,usize,bool,Vec<String>)> { &self.__horizon_mixins_tconstruct_all_state().builder_scroll }
    pub fn __horizon_mixins_tconstruct_builder_scroll_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(f32,usize,bool,Vec<String>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_scroll }
    pub fn __horizon_mixins_tconstruct_builder_preview_sent(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>)> { &self.__horizon_mixins_tconstruct_all_state().builder_preview_sent }
    pub fn __horizon_mixins_tconstruct_builder_preview_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_preview_sent }
    pub fn __horizon_mixins_tconstruct_source_recipe_generation(&self) -> &u64 { &self.__horizon_mixins_tconstruct_all_state().source_recipe_generation }
    pub fn __horizon_mixins_tconstruct_source_recipe_generation_mut(&mut self) -> &mut u64 { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_recipe_generation }
    pub fn __horizon_mixins_tconstruct_builder_recipe_context(&self) -> &std::collections::HashMap<(i32,i32,i32),(u64,std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>,Vec<(String,String)>)> { &self.__horizon_mixins_tconstruct_all_state().builder_recipe_context }
    pub fn __horizon_mixins_tconstruct_builder_recipe_context_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(u64,std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>,Vec<(String,String)>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_recipe_context }
    pub fn __horizon_mixins_tconstruct_source_shared_recipes(&self) -> &Option<std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>> { &self.__horizon_mixins_tconstruct_all_state().source_shared_recipes }
    pub fn __horizon_mixins_tconstruct_source_shared_recipes_mut(&mut self) -> &mut Option<std::sync::Arc<std::collections::BTreeMap<String,Vec<u8>>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_shared_recipes }
    pub fn __horizon_mixins_tconstruct_builder_material_cache(&self) -> &std::collections::HashMap<(i32,i32,i32),(horizon_items::items::ItemStack,Vec<u8>)> { &self.__horizon_mixins_tconstruct_all_state().builder_material_cache }
    pub fn __horizon_mixins_tconstruct_builder_material_cache_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(horizon_items::items::ItemStack,Vec<u8>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_material_cache }
    pub fn __horizon_mixins_tconstruct_source_selection_origin(&self) -> &std::collections::HashMap<(i32,i32,i32),(String,String,u64)> { &self.__horizon_mixins_tconstruct_all_state().source_selection_origin }
    pub fn __horizon_mixins_tconstruct_source_selection_origin_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(String,String,u64)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().source_selection_origin }
    pub fn __horizon_mixins_tconstruct_builder_side(&self) -> &std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().builder_side }
    pub fn __horizon_mixins_tconstruct_builder_side_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_side }
    pub fn __horizon_mixins_tconstruct_builder_side_size(&self) -> &std::collections::HashMap<(i32,i32,i32),usize> { &self.__horizon_mixins_tconstruct_all_state().builder_side_size }
    pub fn __horizon_mixins_tconstruct_builder_side_size_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),usize> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_side_size }
    pub fn __horizon_mixins_tconstruct_builder_double_partner(&self) -> &std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().builder_double_partner }
    pub fn __horizon_mixins_tconstruct_builder_double_partner_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_double_partner }
    pub fn __horizon_mixins_tconstruct_station_preview_sent(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<String>)> { &self.__horizon_mixins_tconstruct_all_state().station_preview_sent }
    pub fn __horizon_mixins_tconstruct_station_preview_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i32,Option<horizon_items::items::ItemStack>,Option<horizon_items::items::ItemStack>,Option<String>,Option<String>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_preview_sent }
    pub fn __horizon_mixins_tconstruct_builder_crafted(&self) -> &Vec<(u32,String,u32,[f64;3])> { &self.__horizon_mixins_tconstruct_all_state().builder_crafted }
    pub fn __horizon_mixins_tconstruct_builder_crafted_mut(&mut self) -> &mut Vec<(u32,String,u32,[f64;3])> { &mut self.__horizon_mixins_tconstruct_all_state_mut().builder_crafted }
    pub fn __horizon_mixins_tconstruct_tool_models(&self) -> &std::sync::OnceLock<Result<std::collections::HashMap<String,Vec<(String,Option<usize>)>>,String>> { &self.__horizon_mixins_tconstruct_all_state().tool_models }
    pub fn __horizon_mixins_tconstruct_tool_models_mut(&mut self) -> &mut std::sync::OnceLock<Result<std::collections::HashMap<String,Vec<(String,Option<usize>)>>,String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().tool_models }
    pub fn __horizon_mixins_tconstruct_tank_fluid_context(&self) -> &Option<Result<(std::collections::HashSet<String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>> { &self.__horizon_mixins_tconstruct_all_state().tank_fluid_context }
    pub fn __horizon_mixins_tconstruct_tank_fluid_context_mut(&mut self) -> &mut Option<Result<(std::collections::HashSet<String>,std::collections::HashMap<String,std::collections::HashSet<String>>),String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().tank_fluid_context }
    pub fn __horizon_mixins_tconstruct_melter_known_items(&self) -> &Option<std::collections::HashSet<String>> { &self.__horizon_mixins_tconstruct_all_state().melter_known_items }
    pub fn __horizon_mixins_tconstruct_melter_known_items_mut(&mut self) -> &mut Option<std::collections::HashSet<String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().melter_known_items }
    pub fn __horizon_mixins_tconstruct_machine_wire_sent(&self) -> &std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag> { &self.__horizon_mixins_tconstruct_all_state().machine_wire_sent }
    pub fn __horizon_mixins_tconstruct_machine_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32, u64, [i32; 3]), horizon_world::nbt::Tag> { &mut self.__horizon_mixins_tconstruct_all_state_mut().machine_wire_sent }
    pub fn __horizon_mixins_tconstruct_remote_screen_position(&self) -> &Option<(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().remote_screen_position }
    pub fn __horizon_mixins_tconstruct_remote_screen_position_mut(&mut self) -> &mut Option<(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().remote_screen_position }
    pub fn __horizon_mixins_tconstruct_station_guest_layout_count(&self) -> &std::collections::HashMap<(u32,i32,(i32,i32,i32)),usize> { &self.__horizon_mixins_tconstruct_all_state().station_guest_layout_count }
    pub fn __horizon_mixins_tconstruct_station_guest_layout_count_mut(&mut self) -> &mut std::collections::HashMap<(u32,i32,(i32,i32,i32)),usize> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_guest_layout_count }
    pub fn __horizon_mixins_tconstruct_station_armor_preview(&self) -> &std::collections::HashMap<(i32,i32,i32),(f32,Option<f32>)> { &self.__horizon_mixins_tconstruct_all_state().station_armor_preview }
    pub fn __horizon_mixins_tconstruct_station_armor_preview_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(f32,Option<f32>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().station_armor_preview }
    pub fn __horizon_mixins_tconstruct_area_guest_hits(&self) -> &std::collections::HashMap<u32,([i32;3],[i32;3],std::collections::HashSet<(i32,i32)>)> { &self.__horizon_mixins_tconstruct_all_state().area_guest_hits }
    pub fn __horizon_mixins_tconstruct_area_guest_hits_mut(&mut self) -> &mut std::collections::HashMap<u32,([i32;3],[i32;3],std::collections::HashSet<(i32,i32)>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().area_guest_hits }
    pub fn __horizon_mixins_tconstruct_area_active(&self) -> &std::collections::HashSet<Option<u32>> { &self.__horizon_mixins_tconstruct_all_state().area_active }
    pub fn __horizon_mixins_tconstruct_area_active_mut(&mut self) -> &mut std::collections::HashSet<Option<u32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().area_active }
    pub fn __horizon_mixins_tconstruct_area_overwield_spent(&self) -> &std::collections::HashSet<Option<u32>> { &self.__horizon_mixins_tconstruct_all_state().area_overwield_spent }
    pub fn __horizon_mixins_tconstruct_area_overwield_spent_mut(&mut self) -> &mut std::collections::HashSet<Option<u32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().area_overwield_spent }
    pub fn __horizon_mixins_tconstruct_transform_guest_hits(&self) -> &std::collections::HashMap<u32,([i32;3],std::collections::HashSet<(i32,i32)>)> { &self.__horizon_mixins_tconstruct_all_state().transform_guest_hits }
    pub fn __horizon_mixins_tconstruct_transform_guest_hits_mut(&mut self) -> &mut std::collections::HashMap<u32,([i32;3],std::collections::HashSet<(i32,i32)>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().transform_guest_hits }
    pub fn __horizon_mixins_tconstruct_melee_context(&self) -> &std::collections::HashMap<Option<u32>,(u32,u32,f32)> { &self.__horizon_mixins_tconstruct_all_state().melee_context }
    pub fn __horizon_mixins_tconstruct_melee_context_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>,(u32,u32,f32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().melee_context }
    pub fn __horizon_mixins_tconstruct_melee_active(&self) -> &std::collections::HashSet<Option<u32>> { &self.__horizon_mixins_tconstruct_all_state().melee_active }
    pub fn __horizon_mixins_tconstruct_melee_active_mut(&mut self) -> &mut std::collections::HashSet<Option<u32>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().melee_active }
    pub fn __horizon_mixins_tconstruct_material_projectile_context(&self) -> &std::collections::HashMap<Option<u32>,(f32,f32,Vec<(f64,f32,bool)>,bool,f64,i32,bool)> { &self.__horizon_mixins_tconstruct_all_state().material_projectile_context }
    pub fn __horizon_mixins_tconstruct_material_projectile_context_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>,(f32,f32,Vec<(f64,f32,bool)>,bool,f64,i32,bool)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().material_projectile_context }
    pub fn __horizon_mixins_tconstruct_armor_wear_context(&self) -> &std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>> { &self.__horizon_mixins_tconstruct_all_state().armor_wear_context }
    pub fn __horizon_mixins_tconstruct_armor_wear_context_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().armor_wear_context }
    pub fn __horizon_mixins_tconstruct_glow_left_held(&self) -> &bool { &self.__horizon_mixins_tconstruct_all_state().glow_left_held }
    pub fn __horizon_mixins_tconstruct_glow_left_held_mut(&mut self) -> &mut bool { &mut self.__horizon_mixins_tconstruct_all_state_mut().glow_left_held }
    pub fn __horizon_mixins_tconstruct_glow_guest_protection(&self) -> &std::collections::HashMap<u32,Option<([i32;3],u32)>> { &self.__horizon_mixins_tconstruct_all_state().glow_guest_protection }
    pub fn __horizon_mixins_tconstruct_glow_guest_protection_mut(&mut self) -> &mut std::collections::HashMap<u32,Option<([i32;3],u32)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().glow_guest_protection }
    pub fn __horizon_mixins_tconstruct_glow_support_checks(&self) -> &std::collections::HashSet<[i32; 3]> { &self.__horizon_mixins_tconstruct_all_state().glow_support_checks }
    pub fn __horizon_mixins_tconstruct_glow_support_checks_mut(&mut self) -> &mut std::collections::HashSet<[i32; 3]> { &mut self.__horizon_mixins_tconstruct_all_state_mut().glow_support_checks }
    pub fn __horizon_mixins_tconstruct_modifier_wear_cause(&self) -> &std::collections::HashMap<Option<u32>, String> { &self.__horizon_mixins_tconstruct_all_state().modifier_wear_cause }
    pub fn __horizon_mixins_tconstruct_modifier_wear_cause_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>, String> { &mut self.__horizon_mixins_tconstruct_all_state_mut().modifier_wear_cause }
    pub fn __horizon_mixins_tconstruct_overslime_pending(&self) -> &std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>> { &self.__horizon_mixins_tconstruct_all_state().overslime_pending }
    pub fn __horizon_mixins_tconstruct_overslime_pending_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>,Vec<(horizon_items::items::ItemStack,u32)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().overslime_pending }
    pub fn __horizon_mixins_tconstruct_materials_book_history(&self) -> &Option<usize> { &self.__horizon_mixins_tconstruct_all_state().materials_book_history }
    pub fn __horizon_mixins_tconstruct_materials_book_history_mut(&mut self) -> &mut Option<usize> { &mut self.__horizon_mixins_tconstruct_all_state_mut().materials_book_history }
    pub fn __horizon_mixins_tconstruct_materials_book_request_sequence(&self) -> &u64 { &self.__horizon_mixins_tconstruct_all_state().materials_book_request_sequence }
    pub fn __horizon_mixins_tconstruct_materials_book_request_sequence_mut(&mut self) -> &mut u64 { &mut self.__horizon_mixins_tconstruct_all_state_mut().materials_book_request_sequence }
    pub fn __horizon_mixins_tconstruct_chest_guest_pages(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(usize,usize)> { &self.__horizon_mixins_tconstruct_all_state().chest_guest_pages }
    pub fn __horizon_mixins_tconstruct_chest_guest_pages_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(usize,usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().chest_guest_pages }
    pub fn __horizon_mixins_tconstruct_chest_ui_pages(&self) -> &std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &self.__horizon_mixins_tconstruct_all_state().chest_ui_pages }
    pub fn __horizon_mixins_tconstruct_chest_ui_pages_mut(&mut self) -> &mut std::collections::HashMap<(i32,i32,i32),(usize,usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().chest_ui_pages }
    pub fn __horizon_mixins_tconstruct_crafting_sessions(&self) -> &std::collections::HashSet<(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().crafting_sessions }
    pub fn __horizon_mixins_tconstruct_crafting_sessions_mut(&mut self) -> &mut std::collections::HashSet<(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_sessions }
    pub fn __horizon_mixins_tconstruct_crafting_guest_sessions(&self) -> &std::collections::HashMap<(u32,u64,i32),(i32,i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().crafting_guest_sessions }
    pub fn __horizon_mixins_tconstruct_crafting_guest_sessions_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32),(i32,i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_guest_sessions }
    pub fn __horizon_mixins_tconstruct_crafting_guest_side(&self) -> &std::collections::HashMap<(u32,u64,i32),((i32,i32,i32),usize)> { &self.__horizon_mixins_tconstruct_all_state().crafting_guest_side }
    pub fn __horizon_mixins_tconstruct_crafting_guest_side_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32),((i32,i32,i32),usize)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_guest_side }
    pub fn __horizon_mixins_tconstruct_crafting_side_client(&self) -> &Option<(i32,usize,usize,Vec<Option<horizon_items::items::ItemStack>>)> { &self.__horizon_mixins_tconstruct_all_state().crafting_side_client }
    pub fn __horizon_mixins_tconstruct_crafting_side_client_mut(&mut self) -> &mut Option<(i32,usize,usize,Vec<Option<horizon_items::items::ItemStack>>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_side_client }
    pub fn __horizon_mixins_tconstruct_menu_window_state(&self) -> &Option<(i32,i32)> { &self.__horizon_mixins_tconstruct_all_state().menu_window_state }
    pub fn __horizon_mixins_tconstruct_menu_window_state_mut(&mut self) -> &mut Option<(i32,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().menu_window_state }
    pub fn __horizon_mixins_tconstruct_crafting_guest_views(&self) -> &std::collections::HashMap<(u32,u64,i32),(Vec<Option<horizon_items::items::ItemStack>>,Vec<Option<horizon_items::items::ItemStack>>)> { &self.__horizon_mixins_tconstruct_all_state().crafting_guest_views }
    pub fn __horizon_mixins_tconstruct_crafting_guest_views_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32),(Vec<Option<horizon_items::items::ItemStack>>,Vec<Option<horizon_items::items::ItemStack>>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_guest_views }
    pub fn __horizon_mixins_tconstruct_chest_guest_drag(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,Vec<(bool,usize)>)> { &self.__horizon_mixins_tconstruct_all_state().chest_guest_drag }
    pub fn __horizon_mixins_tconstruct_chest_guest_drag_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,Vec<(bool,usize)>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().chest_guest_drag }
    pub fn __horizon_mixins_tconstruct_crafting_guest_drag(&self) -> &std::collections::HashMap<(u32,u64,i32),(u8,Vec<(bool,usize)>)> { &self.__horizon_mixins_tconstruct_all_state().crafting_guest_drag }
    pub fn __horizon_mixins_tconstruct_crafting_guest_drag_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32),(u8,Vec<(bool,usize)>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_guest_drag }
    pub fn __horizon_mixins_tconstruct_chest_guest_last_move(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,horizon_items::items::ItemStack)> { &self.__horizon_mixins_tconstruct_all_state().chest_guest_last_move }
    pub fn __horizon_mixins_tconstruct_chest_guest_last_move_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(u8,horizon_items::items::ItemStack)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().chest_guest_last_move }
    pub fn __horizon_mixins_tconstruct_crafting_guest_last_move(&self) -> &std::collections::HashMap<(u32,u64,i32),(u8,horizon_items::items::ItemStack)> { &self.__horizon_mixins_tconstruct_all_state().crafting_guest_last_move }
    pub fn __horizon_mixins_tconstruct_crafting_guest_last_move_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32),(u8,horizon_items::items::ItemStack)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().crafting_guest_last_move }
    pub fn __horizon_mixins_tconstruct_menu_wire_sent(&self) -> &std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i16,Vec<i16>)> { &self.__horizon_mixins_tconstruct_all_state().menu_wire_sent }
    pub fn __horizon_mixins_tconstruct_menu_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64,i32,(i32,i32,i32)),(i16,Vec<i16>)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().menu_wire_sent }
    pub fn __horizon_mixins_tconstruct_necrotic_context(&self) -> &std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>> { &self.__horizon_mixins_tconstruct_all_state().necrotic_context }
    pub fn __horizon_mixins_tconstruct_necrotic_context_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().necrotic_context }
    pub fn __horizon_mixins_tconstruct_necrotic_projectile(&self) -> &std::collections::HashMap<Option<u32>, (u32, f32)> { &self.__horizon_mixins_tconstruct_all_state().necrotic_projectile }
    pub fn __horizon_mixins_tconstruct_necrotic_projectile_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>, (u32, f32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().necrotic_projectile }
    pub fn __horizon_mixins_tconstruct_necrotic_pvp(&self) -> &Vec<Option<(u32, Option<u32>, [i32; 4], Option<f32>, bool)>> { &self.__horizon_mixins_tconstruct_all_state().necrotic_pvp }
    pub fn __horizon_mixins_tconstruct_necrotic_pvp_mut(&mut self) -> &mut Vec<Option<(u32, Option<u32>, [i32; 4], Option<f32>, bool)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().necrotic_pvp }
    pub fn __horizon_mixins_tconstruct_necrotic_host_victim(&self) -> &std::collections::HashMap<u32, (u32, [i32; 4], f32)> { &self.__horizon_mixins_tconstruct_all_state().necrotic_host_victim }
    pub fn __horizon_mixins_tconstruct_necrotic_host_victim_mut(&mut self) -> &mut std::collections::HashMap<u32, (u32, [i32; 4], f32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().necrotic_host_victim }
    pub fn __horizon_mixins_tconstruct_spilling_context(&self) -> &std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>> { &self.__horizon_mixins_tconstruct_all_state().spilling_context }
    pub fn __horizon_mixins_tconstruct_spilling_context_mut(&mut self) -> &mut std::collections::HashMap<Option<u32>, Vec<Option<(u32, horizon_items::items::ItemStack)>>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().spilling_context }
    pub fn __horizon_mixins_tconstruct_splashing_guest_scope(&self) -> &std::collections::HashMap<u32, Vec<(Option<u32>, i32, [f64; 3], [f64; 3], [f64; 3], bool)>> { &self.__horizon_mixins_tconstruct_all_state().splashing_guest_scope }
    pub fn __horizon_mixins_tconstruct_splashing_guest_scope_mut(&mut self) -> &mut std::collections::HashMap<u32, Vec<(Option<u32>, i32, [f64; 3], [f64; 3], [f64; 3], bool)>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().splashing_guest_scope }
    pub fn __horizon_mixins_tconstruct_splashing_guest_pending(&self) -> &std::collections::HashMap<u32, (horizon_items::items::ItemStack, usize, Vec<u32>, Vec<u32>, bool, i32, bool)> { &self.__horizon_mixins_tconstruct_all_state().splashing_guest_pending }
    pub fn __horizon_mixins_tconstruct_splashing_guest_pending_mut(&mut self) -> &mut std::collections::HashMap<u32, (horizon_items::items::ItemStack, usize, Vec<u32>, Vec<u32>, bool, i32, bool)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().splashing_guest_pending }
    pub fn __horizon_mixins_tconstruct_splashing_guest_packet(&self) -> &std::collections::HashMap<u32, (i32, bool)> { &self.__horizon_mixins_tconstruct_all_state().splashing_guest_packet }
    pub fn __horizon_mixins_tconstruct_splashing_guest_packet_mut(&mut self) -> &mut std::collections::HashMap<u32, (i32, bool)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().splashing_guest_packet }
    pub fn __horizon_mixins_tconstruct_haste_wire_client(&self) -> &Option<(horizon_items::items::ItemStack,i32)> { &self.__horizon_mixins_tconstruct_all_state().haste_wire_client }
    pub fn __horizon_mixins_tconstruct_haste_wire_client_mut(&mut self) -> &mut Option<(horizon_items::items::ItemStack,i32)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().haste_wire_client }
    pub fn __horizon_mixins_tconstruct_haste_wire_sent(&self) -> &std::collections::HashMap<(u32,u64),(u64,bool,Option<horizon_items::items::ItemStack>,(Vec<u8>,u8))> { &self.__horizon_mixins_tconstruct_all_state().haste_wire_sent }
    pub fn __horizon_mixins_tconstruct_haste_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64),(u64,bool,Option<horizon_items::items::ItemStack>,(Vec<u8>,u8))> { &mut self.__horizon_mixins_tconstruct_all_state_mut().haste_wire_sent }
    pub fn __horizon_mixins_tconstruct_broad_wire_client(&self) -> &Option<std::collections::HashSet<String>> { &self.__horizon_mixins_tconstruct_all_state().broad_wire_client }
    pub fn __horizon_mixins_tconstruct_broad_wire_client_mut(&mut self) -> &mut Option<std::collections::HashSet<String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().broad_wire_client }
    pub fn __horizon_mixins_tconstruct_broad_wire_sent(&self) -> &std::collections::HashMap<(u32,u64),(Vec<u8>,u8)> { &self.__horizon_mixins_tconstruct_all_state().broad_wire_sent }
    pub fn __horizon_mixins_tconstruct_broad_wire_sent_mut(&mut self) -> &mut std::collections::HashMap<(u32,u64),(Vec<u8>,u8)> { &mut self.__horizon_mixins_tconstruct_all_state_mut().broad_wire_sent }
    pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self) -> &Option<Result<(std::collections::BTreeMap<String,Vec<Vec<u8>>>,std::collections::BTreeMap<String,Vec<Vec<u8>>>),String>> { &self.__horizon_mixins_tconstruct_all_state().modifier_resource_snapshot }
    pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot_mut(&mut self) -> &mut Option<Result<(std::collections::BTreeMap<String,Vec<Vec<u8>>>,std::collections::BTreeMap<String,Vec<Vec<u8>>>),String>> { &mut self.__horizon_mixins_tconstruct_all_state_mut().modifier_resource_snapshot }
    pub fn __horizon_mixins_tconstruct_lan_craft_recipe_generation(&self) -> &u64 { &self.__horizon_mixins_tconstruct_all_state().lan_craft_recipe_generation }
    pub fn __horizon_mixins_tconstruct_lan_craft_recipe_generation_mut(&mut self) -> &mut u64 { &mut self.__horizon_mixins_tconstruct_all_state_mut().lan_craft_recipe_generation }
    pub fn __horizon_mixins_tconstruct_lan_craft_menu_generation(&self) -> &std::collections::HashMap<(u32, u64, i32), u64> { &self.__horizon_mixins_tconstruct_all_state().lan_craft_menu_generation }
    pub fn __horizon_mixins_tconstruct_lan_craft_menu_generation_mut(&mut self) -> &mut std::collections::HashMap<(u32, u64, i32), u64> { &mut self.__horizon_mixins_tconstruct_all_state_mut().lan_craft_menu_generation }
}
