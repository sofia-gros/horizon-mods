//! Short-use contract probe. This is not registered with a running host.
use pluto_item_api::{ItemData, UseOutcome};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Main,
    Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: u128,
    pub dimension: String,
    pub position: [f64; 3],
    pub permission: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockTarget {
    pub position: [i32; 3],
    pub face: [i32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct UseContext {
    pub actor: Actor,
    pub hand: Hand,
    pub item_id: String,
    pub count: u32,
    pub target: Option<BlockTarget>,
}

/// A requested world action. The host adapter must validate and apply actions
/// only on its authoritative side; this prototype does not apply them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldAction {
    SetBlock { position: [i32; 3], block: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedUse {
    pub data: ItemData,
    pub actions: Vec<WorldAction>,
}

/// Run a handler on an isolated draft. Only Handled returns a draft for commit.
/// The caller must validate the original actor, hand and stack before applying it.
pub fn stage_use(
    context: &UseContext,
    data: &ItemData,
    handler: impl FnOnce(&UseContext, &mut StagedUse) -> UseOutcome,
) -> (UseOutcome, Option<StagedUse>) {
    let mut staged = StagedUse {
        data: data.clone(),
        actions: Vec::new(),
    };
    let outcome = handler(context, &mut staged);
    let commit = (outcome == UseOutcome::Handled).then_some(staged);
    (outcome, commit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_handled_returns_data_and_world_actions() {
        let context = UseContext {
            actor: Actor {
                id: 42,
                dimension: "minecraft:overworld".into(),
                position: [1.0, 2.0, 3.0],
                permission: 0,
            },
            hand: Hand::Off,
            item_id: "example:wand".into(),
            count: 1,
            target: Some(BlockTarget {
                position: [4, 5, 6],
                face: [0, 1, 0],
            }),
        };
        let mut original = ItemData::default();
        original.set("example:value", 3);
        for expected in [UseOutcome::Pass, UseOutcome::Cancel, UseOutcome::Handled] {
            let (outcome, draft) = stage_use(&context, &original, |ctx, draft| {
                assert_eq!(ctx.hand, Hand::Off);
                assert_eq!(ctx.target.unwrap().position, [4, 5, 6]);
                draft.data.set("example:value", 4);
                draft.actions.push(WorldAction::SetBlock {
                    position: [4, 6, 6],
                    block: "minecraft:stone".into(),
                });
                expected
            });
            assert_eq!(outcome, expected);
            assert_eq!(original.get("example:value"), Some(3));
            if expected == UseOutcome::Handled {
                let draft = draft.unwrap();
                assert_eq!(draft.data.get("example:value"), Some(4));
                assert_eq!(draft.actions.len(), 1);
            } else {
                assert!(draft.is_none());
            }
        }
    }
}
