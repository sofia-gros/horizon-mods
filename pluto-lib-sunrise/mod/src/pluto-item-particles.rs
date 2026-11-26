// Registered item factories are resolved from this Game's current Ground item
// renderer. Re-enter the native API with ordinary Item options, so its validity,
// client/world, distance/settings/population and RNG policies remain native.
use std::ops::ControlFlow;
use std::sync::OnceLock;
use super::Game;
use sunrise_client::items::ItemStack;

use sunrise_client::particles::ParticleOptions;

use sunrise_client::mobs::MobEvent;
type Query=fn(&str)->Option<ItemStack>;
static QUERY:OnceLock<Query>=OnceLock::new();
pub fn set_provider(query:Query)->Result<(), &'static str> {
    QUERY.set(query).map_err(|_|"item particle factory provider already installed")
}
pub(crate) fn consume(game:&mut Game<'_>,event:&MobEvent) {
    if let MobEvent::Particles {kind,at,count:0}=event {
        // Native from_name only recognizes bare or minecraft: names. Requiring
        // a custom namespace in the override registry prevents double emission.
        if !kind.contains(':') || kind.starts_with("minecraft:") {return;}
        let options=ParticleOptions::named(*kind);
        let _=apply(game,&options,*at,[0.0;3]);
    }
}
pub(crate) fn apply(game:&mut Game<'_>,options:&ParticleOptions,at:[f64;3],velocity:[f64;3])->ControlFlow<bool> {
    let Some(name)=options.mod_particle.as_deref() else {return ControlFlow::Continue(())};
    let Some(stack)=QUERY.get().and_then(|query|query(name)) else {return ControlFlow::Continue(())};
    let converted=game.item_particle(&stack);
    // item_particle constructs Item options with no mod_particle; the recursive
    // HEAD therefore delegates once, not another factory lookup or RNG draw.
    ControlFlow::Break(game.native_add_particle(&converted,at,velocity))
}
