// Records a combat hook's hit before the hit lands: the entity it hooked and the speed it struck
// at, which the hit itself zeroes. The rod wear of a hit (the original's `damageRod`) is reported
// here too. A deflected hit hooks nothing and records nothing.
fn pluto_fishing_hit_head(hook: &mut horizon_client::game::fishing::Hook, id: u32, deflects: bool) {
    if deflects {
        return;
    }
    let Some(active) = crate::entry::pluto_fishing_access::active() else { return };
    if hook.id != active.hook_id {
        return;
    }
    let velocity = hook.velocity;
    let speed = (velocity.x * velocity.x + velocity.y * velocity.y + velocity.z * velocity.z).sqrt() as f32;
    crate::entry::pluto_fishing_access::note_hit(id, speed);
    if let Some(provider) = crate::entry::pluto_fishing_access::provider() {
        (provider.note_wear)(active.uuid, 1);
    }
}
