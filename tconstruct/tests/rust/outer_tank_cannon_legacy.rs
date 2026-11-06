    pub(super) fn tank_to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        let before = Self::from_nbt(raw)?;
        if Tank::from_nbt(&self.tank.to_nbt(), 2000).as_ref() != Some(&self.tank) { return None; }
        if !self.lossless() || self.item.as_ref().is_some_and(|item| item.data().is_some_and(|data| data.wire.patch().is_some())) { return None; }
        match (&before.item, &self.item) {
            (None, None) => {},
            (Some(old), Some(new)) if old.count == new.count && old.damage == new.damage && old.same_item_same_components(new) => {},
            _ => return None,
        }
        for alias in ["Tank", "Fluid"] { if raw.get(alias).is_some() { return None; } }
        let raw_tank = raw.get("tank")?;
        for alias in ["FluidName", "Amount", "Tag"] { if raw_tank.get(alias).is_some() { return None; } }
        if before.tank.to_nbt_preserving(raw_tank).as_ref() != Some(raw_tank) { return None; }
        let tank = self.tank.to_nbt_preserving(raw_tank)?;
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank);
        Some(Tag::Compound(fields))
    }
