// Explicit synchronous pair staging over real borrowed native input stacks.
// No automatic recipe hooks, registry, cached recipe identity or live writes.
use super::Input;
use crate::{
    items::{ItemStack, Items},
    pluto_component_view::{ItemFacts, to_nbt},
    pluto_nbt_storage, pluto_stack_storage,
};
use horizon_world::nbt::Tag;
use pluto_inventory_api::crafting::products::{
    self, CompoundPatch, Error, Grid, MissingCompound, Pair, Preparer, Product, Proposal, Source,
};

pub struct Preparation<'a> {
    source: &'a [Option<ItemStack>],
    input: Input<'a>,
    grid: Grid,
    items: &'a Items,
    views: Vec<Option<ItemFacts>>,
}
/// One owned pair and a full native input snapshot. This is a value comparison
/// receipt, not a world identity, inventory lock or authority to consume slots.
pub struct Prepared {
    source: Vec<Option<ItemStack>>,
    pair: Pair<ItemStack>,
}
impl Prepared {
    /// Release only after the caller supplies its current input values. This
    /// does not establish where they came from or make later writes atomic.
    pub fn release_if_unchanged(
        self,
        current: Option<&[Option<ItemStack>]>,
    ) -> Result<Pair<ItemStack>, Error> {
        if !pluto_stack_storage::inventory_snapshot_matches(&self.source, current) {
            return Err(Error::StaleInput);
        }
        Ok(self.pair)
    }
}
impl<'a> Preparation<'a> {
    pub fn new(
        source: &'a [Option<ItemStack>],
        side: usize,
        items: &'a Items,
    ) -> Result<Self, Error> {
        let cells = side.checked_mul(side).ok_or(Error::InvalidGrid)?;
        if cells == 0 || cells > products::MAX_CELLS || source.len() != cells {
            return Err(Error::InvalidGrid);
        }
        for stack in source.iter().flatten() {
            if stack.count == 0 {
                return Err(Error::InvalidCount);
            }
            if pluto_stack_storage::contains_retained_wire(stack) {
                return Err(Error::OpaqueInput);
            }
        }
        let input = Input::of(source, side).ok_or(Error::EmptyInput)?;
        let grid = Grid {
            side,
            width: input.width,
            height: input.height,
            left: input.left,
            top: input.top,
        };
        grid.cells()?;
        let views = input
            .cells
            .iter()
            .map(|v| v.map(ItemFacts::capture))
            .collect();
        Ok(Self {
            source,
            input,
            grid,
            items,
            views,
        })
    }
    fn derive(&self, product: &Product) -> Result<ItemStack, Error> {
        let mut stack = match &product.source {
            Source::Input(index) => {
                let source = self
                    .input
                    .cells
                    .get(*index)
                    .ok_or(Error::InvalidSource)?
                    .ok_or(Error::EmptySource)?;
                // Validate save eligibility, but never use a decoded copy as a clone.
                pluto_stack_storage::encode_item_lossless(source)
                    .ok_or(Error::UnrepresentableProduct)?;
                source.clone()
            }
            Source::Fresh(id) => ItemStack::new(id.as_str(), product.count),
        };
        let info = self
            .items
            .items
            .get(&stack.item)
            .ok_or(Error::UnknownItem)?;
        if product.count == 0 || product.count > i32::MAX as u32 {
            return Err(Error::InvalidCount);
        }
        stack.count = product.count;
        for patch in &product.patches {
            patch_preserved(&mut stack, patch)?;
        }
        if product.count > effective_limit(&stack, info.max_stack) {
            return Err(Error::InvalidCount);
        }
        pluto_stack_storage::encode_item_lossless(&stack).ok_or(Error::UnrepresentableProduct)?;
        Ok(stack)
    }
}
impl Preparer for Preparation<'_> {
    type Prepared = Prepared;
    fn grid(&self) -> Grid {
        self.grid
    }
    fn cell(
        &self,
        index: usize,
    ) -> Result<Option<pluto_inventory_api::advancements::predicates::ItemSnapshot<'_>>, Error>
    {
        self.views
            .get(index)
            .map(|v| v.as_ref().map(ItemFacts::view))
            .ok_or(Error::InvalidSource)
    }
    fn stage(&self, proposal: &Proposal) -> Result<Prepared, Error> {
        products::validate(proposal, self.grid)?;
        for (index, remainder) in proposal.remainders.iter().enumerate() {
            if remainder.is_some() && self.input.cells[index].is_none() {
                return Err(Error::EmptyRemainderCell);
            }
        }
        let mut pair = products::stage_pair(proposal, self.grid, |p| self.derive(p))?;
        pair.remainders = products::expand(self.grid, pair.remainders)?;
        Ok(Prepared {
            source: self.source.to_vec(),
            pair,
        })
    }
}
// Exact native Menu::max_stack override/removal calculation. The caller has
// already resolved the actual item, so no unknown-item fallback is needed.
fn effective_limit(stack: &ItemStack, item_limit: u32) -> u32 {
    let patched = stack.data().and_then(|d| {
        if d.other("!minecraft:max_stack_size").is_some() {
            return None;
        }
        match d.other("minecraft:max_stack_size") {
            Some(horizon_world::nbt::Tag::Int(n)) if (1..=99).contains(n) => Some(*n as u32),
            _ => None,
        }
    });
    patched.unwrap_or(item_limit)
}
fn patch_preserved(stack: &mut ItemStack, patch: &CompoundPatch) -> Result<(), Error> {
    // Use actual preserved entries, not `other()`'s special modelled-name view.
    let before = stack
        .data()
        .and_then(|d| d.other.iter().find(|(id, _)| id == &patch.component))
        .map(|(_, value)| value.clone());
    let before = match before {
        Some(value) => value,
        None => {
            // An already encoded native modelled field is not a raw other entry.
            if crate::item_data::encode_components(stack)
                .as_ref()
                .is_some_and(|c| c.get(&patch.component).is_some())
            {
                return Err(Error::UnsupportedComponent);
            }
            match patch.missing {
                MissingCompound::Reject => return Err(Error::MissingCompound),
                MissingCompound::Create => Tag::compound([] as [(&str, Tag); 0]),
            }
        }
    };
    let updated = pluto_nbt_storage::patch_compound(
        &before,
        patch.fields.iter().map(|(key, value)| {
            (
                horizon_world::nbt::JavaString(key.0.clone()),
                value.as_ref().map(to_nbt),
            )
        }),
    )
    .ok_or(Error::NonCompound)?;
    let data = stack.data_mut();
    if let Some((_, value)) = data.other.iter_mut().find(|(id, _)| id == &patch.component) {
        *value = updated.clone();
    } else {
        data.other.push((patch.component.clone(), updated.clone()));
        data.other.sort_by(|a, b| a.0.cmp(&b.0));
    }
    // A modelled/conditional/canonicalized component must not silently absorb
    // the patch. Require both exact requested raw value and a native roundtrip.
    if crate::item_data::encode_components(stack)
        .as_ref()
        .and_then(|c| c.get(&patch.component))
        .is_none_or(|v| v != &updated)
        || pluto_stack_storage::encode_item_lossless(stack).is_none()
    {
        return Err(Error::UnrepresentableProduct);
    }
    Ok(())
}
/// Keep host paths in thin adapter glue; policy works with the generic Preparer.
pub fn with_grid<R>(
    source: &[Option<ItemStack>],
    side: usize,
    items: &Items,
    run: impl FnOnce(&Preparation<'_>) -> R,
) -> Result<R, Error> {
    Ok(run(&Preparation::new(source, side, items)?))
}
