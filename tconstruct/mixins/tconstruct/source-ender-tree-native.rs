// Source Ender tree placement at the host tree's root/decorator boundaries.
use super::*;
use super::tconstruct_ender_tree_components::{self as source, VineWorld};

pub struct EnderTree { tree: TreeFeature, variants: std::collections::HashMap<BlockStateId,BlockStateId>, vine: BlockStateId }
impl EnderTree {
    pub fn decode(json: &str, bindings: &FeatureBlockBindings<'_>) -> Result<Self,DecodeError> {
        let registry=bindings.registry();
        let mut remaining=4096;
        let tree=TreeFeature::decode(json,bindings,0,&mut remaining,&|_,_,_|Err(DecodeError::Field("Ender nested feature")))?;
        let mut variants=std::collections::HashMap::new();
        for i in 0..registry.len() {
            let id=BlockStateId(i as u32);
            let Some(state)=registry.state(id) else {continue;};
            if let Some(target)=source::root_variant(&state.name) {
                variants.insert(id,registry.default_state(target).ok_or_else(||DecodeError::Registry(format!("missing {target}")))?);
            }
        }
        let vine=registry.default_state("tconstruct:ender_slime_vine").ok_or_else(||DecodeError::Registry("missing Ender vine".into()))?;
        Ok(Self {tree,variants,vine})
    }
    fn roots<W:FeatureWorld+PlacementContext,R:RandomSource>(&self,root:&RootPlacer,run:&mut Run<'_,W,R>,origin:[i32;3],trunk:[i32;3])->TreeResult<bool,W::Error> {
        let mut column=origin;
        while column[1]<trunk[1] {if !root.can_place_root(run,&self.tree,column)? {return Ok(false);}column[1]=column[1].wrapping_add(1);}
        let mut roots=vec![tb::offset(trunk,0,-1,0)];
        for direction in tb::HORIZONTAL {
            let start=tb::relative(trunk,direction,1);let mut positions=Vec::new();
            if !root.simulate(run,&self.tree,start,direction,trunk,&mut positions,0)? {return Ok(false);}
            roots.extend(positions);roots.push(start);
        }
        for p in roots {
            let old=read(run.world,p)?;
            if let Some(state)=self.variants.get(&old).copied() {
                let state=RootPlacer::waterlogged(&self.tree,run.world,p,state)?;
                run.set(Layer::Root,p,state)?;
            } else {root.place_root(run,&self.tree,p)?;}
        }
        Ok(true)
    }
    pub fn place<W:FeatureWorld+PlacementContext,R:RandomSource>(&self,world:&mut W,random:&mut R,origin:[i32;3]) -> Result<bool,FeatureExecutionError<W::Error>> {
        let tree=&self.tree;
        let mut run=Run {world,random,sets:Sets::default(),track:true};
        let height=tree.trunk_placer.tree_height(run.random)?;
        let foliage_height=tree.foliage_placer.foliage_height(run.random,height)?;
        let leaf_radius=tree.foliage_placer.foliage_radius(run.random,height.wrapping_sub(foliage_height))?;
        let root=tree.root_placer.as_ref().ok_or(FeatureExecutionError::InvalidState)?;
        let trunk=tb::offset(origin,0,root.trunk_offset_y.sample(run.random)?,0);
        let min_y=origin[1].min(trunk[1]);let max_y=origin[1].max(trunk[1]).wrapping_add(height).wrapping_add(1);
        let min=run.world.min_y().map_err(FeatureExecutionError::Query)?;let max=run.world.max_y().map_err(FeatureExecutionError::Query)?;
        if min_y<min.wrapping_add(1)||max_y>max.wrapping_add(1) {return Ok(false);}
        let clipped=tree.max_free_tree_height(&mut run,height,trunk)?;
        if clipped<height && tree.min_clipped_height.is_none_or(|min|clipped<min) {return Ok(false);}
        if !self.roots(root,&mut run,origin,trunk)? {return Ok(false);}
        let attachments=tree.trunk_placer.place_trunk(&mut run,tree,clipped,trunk)?;
        for attachment in attachments {tree.foliage_placer.create_foliage(&mut run,tree,clipped,attachment,foliage_height,leaf_radius)?;}
        if run.sets.trunks.is_empty() && run.sets.foliage.is_empty() {return Ok(false);}
        let lists=decorator::Lists::new(run.sets.trunks.to_vec(),run.sets.foliage.to_vec(),run.sets.roots.to_vec());
        {let mut vines=Vines {run:&mut run,blocks:&tree.blocks,vine:self.vine};source::decorate(&mut vines,&lists.leaves,0.125)?;}
        let mut nested=|_:&FeatureDefinition,_:&mut W,_:&mut R,_:[i32;3]|->TreeResult<bool,W::Error>{Ok(false)};
        for decorator in &tree.decorators {decorator.place(&mut run,&tree.blocks,&lists,&mut nested)?;}
        let mut bounds:Option<([i32;3],[i32;3])>=None;
        for set in [&run.sets.roots,&run.sets.trunks,&run.sets.foliage,&run.sets.decorations] {for p in set.to_vec() {
            bounds=Some(match bounds {None=>(p,p),Some((lo,hi))=>(std::array::from_fn(|i|lo[i].min(p[i])),std::array::from_fn(|i|hi[i].max(p[i])))});
        }}
        let Some((low,high))=bounds else {return Ok(false);};
        let shape=update_leaves(&mut run,&tree.blocks,low,high)?;update_shape_at_edge(run.world,&shape,low,3)?;
        Ok(true)
    }
}
struct Vines<'a,'b,W,R> {run:&'a mut Run<'b,W,R>,blocks:&'a TreeBlocks,vine:BlockStateId}
impl<W:FeatureWorld+PlacementContext,R:RandomSource> VineWorld for Vines<'_,'_,W,R> {
    type Error=FeatureExecutionError<W::Error>;
    fn next_float(&mut self)->f32 {self.run.random.next_float()}
    fn is_air(&mut self,pos:[i32;3])->Result<bool,Self::Error> {Ok(self.blocks.is_air(read(self.run.world,pos)?))}
    fn place_vine(&mut self,pos:[i32;3],face:&'static str)->Result<(),Self::Error> {
        let state=self.blocks.with(self.vine,face,"true").ok_or(FeatureExecutionError::InvalidState)?;
        self.run.set(Layer::Decoration,pos,state)
    }
}
