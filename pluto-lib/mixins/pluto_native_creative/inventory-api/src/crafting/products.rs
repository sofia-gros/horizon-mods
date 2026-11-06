//! Explicit synchronous output/remainder proposals, never automatic crafting.
//! Staging an entire pair does not commit inventory or prove slot authority.
use crate::advancements::predicates::{ComponentText, ComponentValue, ItemSnapshot};

pub const MAX_CELLS: usize = 4096;
pub const MAX_PATCHES: usize = 4096;
pub const MAX_VALUE_NODES: usize = 65_536;
pub const MAX_VALUE_DEPTH: usize = 64;
pub const MAX_TEXT_UNITS: usize = 1024;
pub const MAX_PROPOSAL_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Source {
    /// Index into the cropped cells, including empty positions.
    Input(usize),
    /// A fresh default stack of a known item; no inherited metadata.
    Fresh(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingCompound {
    Reject,
    Create,
}
#[derive(Clone, Debug)]
pub struct CompoundPatch {
    /// A preserved component, not a native modelled field.
    pub component: String,
    pub missing: MissingCompound,
    /// Explicit ordered top-level field changes; None removes a field.
    /// Unknown fields are retained. Nested value replacement is caller policy.
    pub fields: Vec<(ComponentText, Option<ComponentValue>)>,
}
#[derive(Clone, Debug)]
pub struct Product {
    pub source: Source,
    pub count: u32,
    pub patches: Vec<CompoundPatch>,
}
#[derive(Clone, Debug)]
pub struct Proposal {
    pub output: Product,
    /// Exactly one entry per cropped cell. A remainder may only occupy an
    /// originally occupied cell. None means no remainder for that cell.
    pub remainders: Vec<Option<Product>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub side: usize,
    pub width: usize,
    pub height: usize,
    pub left: usize,
    pub top: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidGrid,
    EmptyInput,
    WrongRemainderCount,
    InvalidSource,
    EmptySource,
    EmptyRemainderCell,
    InvalidCount,
    LimitExceeded,
    OpaqueInput,
    UnknownItem,
    UnsupportedComponent,
    MissingCompound,
    NonCompound,
    UnrepresentableProduct,
    StaleInput,
}
impl Grid {
    pub fn cells(self) -> Result<usize, Error> {
        let full = self.side.checked_mul(self.side).ok_or(Error::InvalidGrid)?;
        let cropped = self
            .width
            .checked_mul(self.height)
            .ok_or(Error::InvalidGrid)?;
        if full == 0
            || full > MAX_CELLS
            || cropped == 0
            || cropped > MAX_CELLS
            || self
                .left
                .checked_add(self.width)
                .is_none_or(|n| n > self.side)
            || self
                .top
                .checked_add(self.height)
                .is_none_or(|n| n > self.side)
        {
            return Err(Error::InvalidGrid);
        }
        Ok(cropped)
    }
    pub fn original_index(self, cropped: usize) -> Result<usize, Error> {
        if cropped >= self.cells()? {
            return Err(Error::InvalidSource);
        }
        Ok((self.top + cropped / self.width) * self.side + self.left + cropped % self.width)
    }
}
/// Fully staged owned values. Neither field has been published to live state.
#[derive(Debug)]
pub struct Pair<T> {
    pub output: T,
    pub remainders: Vec<Option<T>>,
}
/// A short-lived borrowed input capability. Facts are observations, not native
/// stack reconstruction data. The adapter retains actual full stacks privately.
pub trait Preparer {
    type Prepared;
    fn grid(&self) -> Grid;
    fn cell(&self, cropped: usize) -> Result<Option<ItemSnapshot<'_>>, Error>;
    fn stage(&self, proposal: &Proposal) -> Result<Self::Prepared, Error>;
}
struct Budget {
    nodes: usize,
    bytes: usize,
    patches: usize,
}
impl Budget {
    fn bytes(&mut self, count: usize) -> Result<(), Error> {
        self.bytes = self.bytes.checked_add(count).ok_or(Error::LimitExceeded)?;
        if self.bytes > MAX_PROPOSAL_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(())
    }
    fn text(&mut self, text: &ComponentText) -> Result<(), Error> {
        if text.0.len() > MAX_TEXT_UNITS {
            return Err(Error::LimitExceeded);
        }
        self.bytes(text.0.len().checked_mul(2).ok_or(Error::LimitExceeded)?)
    }
    fn value(&mut self, root: &ComponentValue) -> Result<(), Error> {
        let mut pending = vec![(root, 0usize)];
        while let Some((value, depth)) = pending.pop() {
            self.nodes = self.nodes.checked_add(1).ok_or(Error::LimitExceeded)?;
            if self.nodes > MAX_VALUE_NODES || depth > MAX_VALUE_DEPTH {
                return Err(Error::LimitExceeded);
            }
            self.bytes(8)?;
            use ComponentValue::*;
            match value {
                String(text) => self.text(text)?,
                ByteArray(v) => self.bytes(v.len())?,
                IntArray(v) => self.bytes(v.len().checked_mul(4).ok_or(Error::LimitExceeded)?)?,
                LongArray(v) => self.bytes(v.len().checked_mul(8).ok_or(Error::LimitExceeded)?)?,
                List(values) => {
                    if values.len()
                        > MAX_VALUE_NODES.saturating_sub(self.nodes.saturating_add(pending.len()))
                    {
                        return Err(Error::LimitExceeded);
                    }
                    pending.extend(values.iter().map(|value| (value, depth + 1)));
                }
                Compound(values) => {
                    if values.len()
                        > MAX_VALUE_NODES.saturating_sub(self.nodes.saturating_add(pending.len()))
                    {
                        return Err(Error::LimitExceeded);
                    }
                    for (key, value) in values {
                        self.text(key)?;
                        pending.push((value, depth + 1));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn product(&mut self, product: &Product, cells: usize) -> Result<(), Error> {
        if product.count == 0 || product.count > i32::MAX as u32 {
            return Err(Error::InvalidCount);
        }
        match &product.source {
            Source::Input(index) if *index >= cells => return Err(Error::InvalidSource),
            Source::Fresh(id) => {
                if id.is_empty() || id.len() > MAX_TEXT_UNITS {
                    return Err(Error::LimitExceeded);
                }
                self.bytes(id.len())?;
            }
            _ => {}
        }
        self.patches = self
            .patches
            .checked_add(product.patches.len())
            .ok_or(Error::LimitExceeded)?;
        if self.patches > MAX_PATCHES {
            return Err(Error::LimitExceeded);
        }
        for patch in &product.patches {
            if patch.component.is_empty() || patch.component.len() > MAX_TEXT_UNITS {
                return Err(Error::LimitExceeded);
            }
            self.bytes(patch.component.len())?;
            self.nodes = self
                .nodes
                .checked_add(patch.fields.len())
                .ok_or(Error::LimitExceeded)?;
            if self.nodes > MAX_VALUE_NODES {
                return Err(Error::LimitExceeded);
            }
            for (key, value) in &patch.fields {
                self.text(key)?;
                if let Some(v) = value {
                    self.value(v)?;
                }
            }
        }
        Ok(())
    }
}
/// Bound and structurally validate the whole proposal before deriving any stack.
/// Bounds cover supplied proposals, not arbitrary consumer allocation or inputs.
pub fn validate(proposal: &Proposal, grid: Grid) -> Result<(), Error> {
    let cells = grid.cells()?;
    if proposal.remainders.len() != cells {
        return Err(Error::WrongRemainderCount);
    }
    let mut budget = Budget {
        nodes: 0,
        bytes: 0,
        patches: 0,
    };
    budget.product(&proposal.output, cells)?;
    for product in proposal.remainders.iter().flatten() {
        budget.product(product, cells)?;
    }
    Ok(())
}
/// Derive independent owned values only. Any error drops the entire staged pair.
/// The callback must not mutate live storage or publish side effects.
pub fn stage_pair<T>(
    proposal: &Proposal,
    grid: Grid,
    mut derive: impl FnMut(&Product) -> Result<T, Error>,
) -> Result<Pair<T>, Error> {
    validate(proposal, grid)?;
    let output = derive(&proposal.output)?;
    let remainders = proposal
        .remainders
        .iter()
        .map(|p| p.as_ref().map(&mut derive).transpose())
        .collect::<Result<_, _>>()?;
    Ok(Pair { output, remainders })
}
/// Move each cropped remainder to its original grid position without cloning.
pub fn expand<T>(grid: Grid, cropped: Vec<Option<T>>) -> Result<Vec<Option<T>>, Error> {
    if cropped.len() != grid.cells()? {
        return Err(Error::WrongRemainderCount);
    }
    let mut full: Vec<Option<T>> = (0..grid.side * grid.side).map(|_| None).collect();
    for (index, value) in cropped.into_iter().enumerate() {
        full[grid.original_index(index)?] = value;
    }
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grid() -> Grid {
        Grid {
            side: 3,
            width: 2,
            height: 2,
            left: 1,
            top: 1,
        }
    }
    fn product() -> Product {
        Product {
            source: Source::Input(0),
            count: 1,
            patches: vec![],
        }
    }
    fn proposal() -> Proposal {
        Proposal {
            output: product(),
            remainders: vec![Some(product()), None, None, Some(product())],
        }
    }
    #[test]
    fn maps_holes_and_offsets_without_duplication() {
        assert_eq!(
            expand(grid(), vec![Some(1), None, None, Some(4)]).unwrap(),
            vec![None, None, None, None, Some(1), None, None, None, Some(4)]
        );
    }
    #[test]
    fn invalid_shape_or_count_never_starts_derivation() {
        let mut p = proposal();
        p.remainders.pop();
        assert_eq!(
            stage_pair::<()>(&p, grid(), |_| panic!()).unwrap_err(),
            Error::WrongRemainderCount
        );
        assert_eq!(grid().original_index(4), Err(Error::InvalidSource));
        assert_eq!(
            Grid {
                side: usize::MAX,
                ..grid()
            }
            .cells(),
            Err(Error::InvalidGrid)
        );
        assert_eq!(Grid { left: 2, ..grid() }.cells(), Err(Error::InvalidGrid));
    }
    #[test]
    fn any_derivation_failure_drops_all_staged_values() {
        use std::cell::Cell;
        struct Owned<'a>(&'a Cell<usize>);
        impl Drop for Owned<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let dropped = Cell::new(0);
        let mut calls = 0;
        let result = stage_pair(&proposal(), grid(), |_| {
            calls += 1;
            if calls == 3 {
                Err(Error::UnrepresentableProduct)
            } else {
                Ok(Owned(&dropped))
            }
        });
        assert!(matches!(result, Err(Error::UnrepresentableProduct)));
        assert_eq!(calls, 3);
        assert_eq!(dropped.get(), 2);
    }
    #[test]
    fn complete_pair_derives_in_stable_order() {
        let mut calls = 0;
        let pair = stage_pair(&proposal(), grid(), |_| {
            calls += 1;
            Ok(calls)
        })
        .unwrap();
        assert_eq!(pair.output, 1);
        assert_eq!(pair.remainders, vec![Some(2), None, None, Some(3)]);
    }
    #[test]
    fn invalid_last_product_prevents_even_output_derivation() {
        let mut p = proposal();
        p.remainders[3].as_mut().unwrap().count = 0;
        assert_eq!(
            stage_pair::<()>(&p, grid(), |_| panic!()).unwrap_err(),
            Error::InvalidCount
        );
        p.remainders[3].as_mut().unwrap().count = 1;
        p.output.source = Source::Input(4);
        assert_eq!(validate(&p, grid()), Err(Error::InvalidSource));
    }
    #[test]
    fn values_are_bounded_before_native_conversion() {
        let mut value = ComponentValue::Int(2);
        for _ in 0..=MAX_VALUE_DEPTH {
            value = ComponentValue::List(vec![value]);
        }
        let mut p = proposal();
        p.output.patches.push(CompoundPatch {
            component: "example:data".into(),
            missing: MissingCompound::Create,
            fields: vec![("x".into(), Some(value))],
        });
        assert_eq!(validate(&p, grid()), Err(Error::LimitExceeded));
        p.output.patches[0].fields[0].1 =
            Some(ComponentValue::ByteArray(vec![0; MAX_PROPOSAL_BYTES]));
        assert_eq!(validate(&p, grid()), Err(Error::LimitExceeded));
    }
    #[test]
    fn raw_utf16_and_numeric_variants_are_not_interpreted() {
        let mut p = proposal();
        p.output.patches.push(CompoundPatch {
            component: "example:data".into(),
            missing: MissingCompound::Reject,
            fields: vec![
                (
                    ComponentText(vec![0xd800]),
                    Some(ComponentValue::Float(0x7fc00001)),
                ),
                ("removed".into(), None),
            ],
        });
        assert_eq!(validate(&p, grid()), Ok(()));
    }
}
