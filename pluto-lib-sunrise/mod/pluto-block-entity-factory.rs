// Cancellable HEAD for Game<'_>::new_block_entity. Manifest and whole-source
// contract are source-integrated, not yet validated against a real combined D
// plan or native build. Predicate installs before resource/world initialization.
fn pluto_block_entity_factory(game: &mut Game<'_>, name: &str) -> ::core::ops::ControlFlow<Option<BlockEntity>> {
    use crate::entry::pluto_block_entity_access::{registered, route_factory};
    match route_factory(name, registered, || {
        // Retain the existing ordered native callback dispatcher. This is called
        // exactly once only for explicitly registered custom blocks, rather than
        // calling it speculatively then falling through to a second invocation.
        let mut entity = None;
        game.native_block_entity(name, &mut entity);
        entity.map(|(id, nbt)| BlockEntity::Mod(Box::new(ModBlockEntityFields {
            id, nbt, ..Default::default()
        }.into())))
    }) {
        ::core::ops::ControlFlow::Continue(()) => ::core::ops::ControlFlow::Continue(()),
        ::core::ops::ControlFlow::Break(Ok(entity)) => ::core::ops::ControlFlow::Break(entity),
        ::core::ops::ControlFlow::Break(Err(error)) => {
            eprintln!("pluto_lib: block entity factory refused {name}: {error}");
            ::core::ops::ControlFlow::Break(None)
        }
    }
}
