fn pluto_living_attributes_reload(value:R<()>,game:&mut Game<'_>,target:Target)->R<()> {
    // A refused native reload must not invoke class callbacks or rewrite bases.
    value?;
    if let Target::Mob(entity)=target {
        if let Err(error)=sunrise_client::game::pluto_living_attributes_join::after_nbt_load(game,entity) {
            log::error!("PlutoLib reloaded class attributes: {error}");
            // Native entity data has already changed; no rollback is promised.
            return Err(err("commands.data.merge.failed",&[]));
        }
    }
    Ok(())
}
