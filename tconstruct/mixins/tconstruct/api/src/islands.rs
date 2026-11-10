//! Original IslandStructure selection and IslandPlacement height rules.
//! This consumer core is not connected to structure generation yet. The host
//! must supply the structure attempt's existing random stream and height queries
//! through PlutoLib; it must not reseed here or scan already-generated chunks.
use pluto_lib::{ConfiguredStructure, Identifier, StructureContext, StructurePiece, StructureStart, StructureTypeRegistry, StructureWorld};
use serde_json::Value;

type Pos = [i32; 3];

/// Narrow consumer-side contract; not a host interface or a public PlutoLib API.
/// Query failures stay failures rather than guessed heights or new random draws.
pub(super) trait Context {
    fn surface_height(&mut self, x: i32, z: i32) -> Result<i32, String>;
    fn generation_depth(&self) -> i32;
    fn sea_level(&self) -> i32;
    fn random_int(&mut self, bound: i32) -> Result<i32, String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rotation { None, Clockwise90, Clockwise180, Counterclockwise90 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mirror { None, LeftRight, FrontBack }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement { Sky, Sea }

fn draw(context: &mut dyn Context, bound: i32) -> Result<i32, String> {
    let value = context.random_int(bound)?;
    if (0..bound).contains(&value) { Ok(value) } else { Err("invalid structure random result".into()) }
}

impl Placement {
    fn height(self, context: &mut dyn Context, chunk: [i32; 2], rotation: Rotation) -> Result<i32, String> {
        if self == Self::Sea { return Ok(context.sea_level().wrapping_sub(7).max(0)); }
        let (dx, dz) = match rotation {
            Rotation::None => (5, 5), Rotation::Clockwise90 => (-5, 5),
            Rotation::Clockwise180 => (-5, -5), Rotation::Counterclockwise90 => (5, -5),
        };
        let x = chunk[0].wrapping_mul(16).wrapping_add(7);
        let z = chunk[1].wrapping_mul(16).wrapping_add(7);
        // Preserve the Java query order and int arithmetic, including negative
        // chunks and its generation-depth cap (not minY + depth).
        let a = context.surface_height(x, z)?;
        let b = context.surface_height(x, z.wrapping_add(dz))?;
        let c = context.surface_height(x.wrapping_add(dx), z)?;
        let d = context.surface_height(x.wrapping_add(dx), z.wrapping_add(dz))?;
        let height = a.min(b).min(c).min(d).wrapping_add(60).wrapping_add(draw(context, 50)?);
        Ok(height.min(context.generation_depth().wrapping_sub(20)))
    }
}

#[derive(Clone)]
struct Choices { entries: Vec<(String, i32)>, total: i32 }
impl Choices {
    fn decode(value: &Value, field: &str) -> Result<Self, String> {
        let entries = value.get(field).and_then(Value::as_array).ok_or_else(|| format!("missing {field} list"))?;
        if entries.len() > 256 { return Err(format!("too many {field} choices")); }
        let mut total = 0i32;
        let entries = entries.iter().map(|entry| {
            let id = entry.get("data").and_then(Value::as_str).ok_or_else(|| format!("invalid {field} identifier"))?;
            Identifier::parse(id).map_err(|_| format!("invalid {field} identifier: {id}"))?;
            let weight = entry.get("weight").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok())
                .filter(|v| *v >= 0).ok_or_else(|| format!("invalid {field} weight"))?;
            total = total.checked_add(weight).ok_or_else(|| format!("{field} weight overflow"))?;
            Ok((id.to_owned(), weight))
        }).collect::<Result<Vec<_>, String>>()?;
        Ok(Self { entries, total })
    }
    fn choose(&self, context: &mut dyn Context) -> Result<Option<&str>, String> {
        // SimpleWeightedRandomList makes no random draw with zero total weight.
        if self.total == 0 { return Ok(None); }
        let mut value = draw(context, self.total)?;
        for (id, weight) in &self.entries {
            value -= weight;
            if value < 0 { return Ok(Some(id)); }
        }
        Err("structure weighted choice exhausted".into())
    }
}

pub(super) struct IslandSpec {
    placement: Placement,
    templates: Choices,
    trees: Choices,
    grasses: Choices,
    vines: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Layout {
    pub template: String,
    pub tree: Option<String>,
    pub origin: Pos,
    pub rotation: Rotation,
    pub mirror: Mirror,
    pub grasses: Vec<(String, i32)>,
    pub vines: Option<String>,
}

impl IslandSpec {
    pub(super) fn decode(value: &Value) -> Result<Self, String> {
        if value.get("type").and_then(Value::as_str) != Some("tconstruct:island") {
            return Err("not an original island structure".into());
        }
        let placement = match value.get("placement").and_then(Value::as_str) {
            Some("sky") => Placement::Sky, Some("sea") => Placement::Sea,
            _ => return Err("invalid island placement".into()),
        };
        let vines = match value.get("vines") {
            None => None,
            Some(Value::String(id)) => {
                Identifier::parse(id).map_err(|_| "invalid island vine identifier".to_owned())?;
                Some(id.clone())
            },
            _ => return Err("invalid island vines".into()),
        };
        Ok(Self { placement, templates: Choices::decode(value, "templates")?,
            trees: Choices::decode(value, "trees")?, grasses: Choices::decode(value, "grasses")?, vines })
    }

    /// Called only after the host's top-of-chunk-centre/biome check. This is a
    /// plan, not placement: no template, marker or entity is written here.
    pub(super) fn layout(&self, context: &mut dyn Context, chunk: [i32; 2]) -> Result<Option<Layout>, String> {
        let Some(template) = self.templates.choose(context)? else { return Ok(None); };
        let rotation = [Rotation::None, Rotation::Clockwise90, Rotation::Clockwise180, Rotation::Counterclockwise90][draw(context, 4)? as usize];
        let height = self.placement.height(context, chunk, rotation)?;
        let origin = [chunk[0].wrapping_mul(16).wrapping_add(8), height, chunk[1].wrapping_mul(16).wrapping_add(8)];
        let mirror = [Mirror::None, Mirror::LeftRight, Mirror::FrontBack][draw(context, 3)? as usize];
        let tree = self.trees.choose(context)?.map(str::to_owned);
        Ok(Some(Layout { template: template.into(), tree, origin, rotation, mirror,
            grasses: self.grasses.entries.clone(), vines: self.vines.clone() }))
    }
}

struct PlanningWorld<'a>(&'a mut dyn StructureWorld);
impl Context for PlanningWorld<'_> {
    fn surface_height(&mut self, x:i32, z:i32)->Result<i32,String> {self.0.first_occupied_height(x,z)}
    fn generation_depth(&self)->i32 {self.0.generation_depth()}
    fn sea_level(&self)->i32 {self.0.sea_level()}
    fn random_int(&mut self,bound:i32)->Result<i32,String> {self.0.random_int(bound)}
}

impl IslandSpec {
    fn plan(&self, context: &mut StructureContext<'_>) -> Result<Option<StructureStart>,String> {
        let Some(position) = context.world.top_of_chunk_center(context.chunk)? else {return Ok(None)};
        if !context.world.is_valid_biome(position)? {return Ok(None)}
        let Some(layout) = self.layout(&mut PlanningWorld(&mut *context.world),context.chunk)? else {return Ok(None)};
        let rotation = match layout.rotation {Rotation::None=>"NONE",Rotation::Clockwise90=>"CLOCKWISE_90",Rotation::Clockwise180=>"CLOCKWISE_180",Rotation::Counterclockwise90=>"COUNTERCLOCKWISE_90"};
        let mirror = match layout.mirror {Mirror::None=>"NONE",Mirror::LeftRight=>"LEFT_RIGHT",Mirror::FrontBack=>"FRONT_BACK"};
        let grasses:Vec<_> = layout.grasses.iter().map(|(id,weight)| serde_json::json!({"data":id,"weight":weight})).collect();
        Ok(Some(StructureStart {position,pieces:vec![StructurePiece {
            kind:Identifier::parse("tconstruct:island_piece").expect("fixed piece type"),
            configuration:serde_json::json!({"template":layout.template,"origin":layout.origin,
                "rotation":rotation,"mirror":mirror,"tree":layout.tree,"grasses":grasses,
                "vines":layout.vines,"trees_placed":0}),
        }]}))
    }
}

/// Register only the configuration/planning codec. Native structure dispatch,
/// piece placement and saved-piece reconstruction are still unconnected.
pub(super) fn register() {
    assert!(StructureTypeRegistry::register(Identifier::parse("tconstruct:island").expect("fixed structure type"),|configuration| {
        let spec=IslandSpec::decode(configuration)?;
        Ok(ConfiguredStructure::new(move |context| spec.plan(context)))
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Queries { heights: VecDeque<Result<i32, String>>, rolls: VecDeque<i32>, points: Vec<(i32, i32)>, bounds: Vec<i32>, depth: i32, sea: i32 }
    impl Context for Queries {
        fn surface_height(&mut self, x: i32, z: i32) -> Result<i32, String> { self.points.push((x,z)); self.heights.pop_front().expect("unexpected height query") }
        fn generation_depth(&self) -> i32 { self.depth }
        fn sea_level(&self) -> i32 { self.sea }
        fn random_int(&mut self, bound: i32) -> Result<i32, String> { self.bounds.push(bound); Ok(self.rolls.pop_front().expect("unexpected random draw")) }
    }
    fn queries(rolls: &[i32]) -> Queries {
        Queries { heights: [70, 80, 65, 90].into_iter().map(Ok).collect(), rolls: rolls.iter().copied().collect(), points: vec![], bounds: vec![], depth: 384, sea: 63 }
    }

    impl StructureWorld for Queries {
        fn top_of_chunk_center(&mut self,chunk:[i32;2])->Result<Option<Pos>,String> {Ok(Some([chunk[0]*16+8,70,chunk[1]*16+8]))}
        fn is_valid_biome(&mut self,_:Pos)->Result<bool,String> {Ok(self.sea>=0)}
        fn first_occupied_height(&mut self,x:i32,z:i32)->Result<i32,String> {Context::surface_height(self,x,z)}
        fn generation_depth(&self)->i32 {self.depth}
        fn sea_level(&self)->i32 {self.sea}
        fn random_int(&mut self,bound:i32)->Result<i32,String> {Context::random_int(self,bound)}
    }
    #[test]
    fn registered_structure_checks_biome_before_piece_draws_and_retains_owned_piece_data() {
        register();
        let source:Value=serde_json::from_str(include_str!("../../../../content/reference/slime-islands/data/tconstruct/worldgen/structure/end_slime_island.json")).unwrap();
        let structure=StructureTypeRegistry::decode(&Identifier::parse("tconstruct:island").unwrap(),&source).unwrap().unwrap();
        let mut context=queries(&[4,1,49,2,3]); context.sea=-1;
        assert_eq!(structure.find_generation_point(&mut StructureContext {world:&mut context,chunk:[-2,3]}),Ok(None));
        assert!(context.bounds.is_empty()); assert!(context.points.is_empty());
        context.sea=63;
        let start=structure.find_generation_point(&mut StructureContext {world:&mut context,chunk:[-2,3]}).unwrap().unwrap();
        assert_eq!(start.position,[-24,70,56]);
        assert_eq!(start.pieces[0].kind.to_string(),"tconstruct:island_piece");
        let data=&start.pieces[0].configuration;
        assert_eq!(data["origin"],serde_json::json!([-24,174,56]));
        assert_eq!(data["rotation"],"CLOCKWISE_90"); assert_eq!(data["mirror"],"FRONT_BACK");
        assert_eq!(data["tree"],"tconstruct:ender_slime_tree_tall"); assert_eq!(data["trees_placed"],0);
        assert_eq!(context.bounds,[5,4,50,3,20]);
    }

    #[test]
    fn original_ender_selection_keeps_draw_order_rotation_height_and_tree_weights() {
        let source: Value = serde_json::from_str(include_str!("../../../../content/reference/slime-islands/data/tconstruct/worldgen/structure/end_slime_island.json")).unwrap();
        let spec = IslandSpec::decode(&source).unwrap();
        let mut context = queries(&[4, 1, 49, 2, 3]);
        let layout = spec.layout(&mut context, [-2, 3]).unwrap().unwrap();
        assert_eq!(context.bounds, [5, 4, 50, 3, 20]);
        assert_eq!(context.points, [(-25,55),(-25,60),(-30,55),(-30,60)]);
        assert_eq!(layout.origin, [-24,174,56]);
        assert_eq!(layout.rotation, Rotation::Clockwise90);
        assert_eq!(layout.mirror, Mirror::FrontBack);
        assert_eq!(layout.template, "tconstruct:islands/ender/11x1x11");
        assert_eq!(layout.tree.as_deref(), Some("tconstruct:ender_slime_tree_tall"));
        assert_eq!(layout.grasses, [("tconstruct:ender_slime_tall_grass".into(),7),("tconstruct:ender_slime_fern".into(),1)]);
        assert!(context.rolls.is_empty());
        // The first three values choose the normal tree, not the tall tree.
        let mut context = queries(&[0, 0, 0, 0, 2]);
        assert_eq!(spec.layout(&mut context, [0,0]).unwrap().unwrap().tree.as_deref(), Some("tconstruct:ender_slime_tree"));
    }

    #[test]
    fn heights_and_empty_choices_preserve_java_boundaries_and_query_failures() {
        for (rotation, corners) in [
            (Rotation::None, [(7,7),(7,12),(12,7),(12,12)]),
            (Rotation::Clockwise90, [(7,7),(7,12),(2,7),(2,12)]),
            (Rotation::Clockwise180, [(7,7),(7,2),(2,7),(2,2)]),
            (Rotation::Counterclockwise90, [(7,7),(7,2),(12,7),(12,2)]),
        ] {
            let mut context = queries(&[49]); context.depth = 128;
            assert_eq!(Placement::Sky.height(&mut context,[0,0],rotation).unwrap(),108);
            assert_eq!(context.points,corners); assert_eq!(context.bounds,[50]);
        }
        let mut context = queries(&[]); context.sea = 4;
        assert_eq!(Placement::Sea.height(&mut context,[0,0],Rotation::None).unwrap(),0);
        assert!(context.points.is_empty()); assert!(context.bounds.is_empty());
        let empty = Choices::decode(&serde_json::json!({"trees":[{"data":"minecraft:oak","weight":0}]}),"trees").unwrap();
        assert_eq!(empty.choose(&mut context).unwrap(),None); assert!(context.bounds.is_empty());
        context.heights = [Err("unavailable column".into())].into();
        assert_eq!(Placement::Sky.height(&mut context,[0,0],Rotation::None),Err("unavailable column".into()));
        assert!(context.bounds.is_empty());
        assert!(Choices::decode(&serde_json::json!({"trees":[{"data":"minecraft:oak","weight":2147483647},{"data":"minecraft:birch","weight":1}]}),"trees").is_err());
    }
}
