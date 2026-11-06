    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        if !self.lossless() || Tank::from_nbt(&self.tank.to_nbt(), 4000).as_ref() != Some(&self.tank) { return None; }
        let before = Self::from_nbt(raw)?;
        for alias in ["Items", "Tank", "Fluid", "lastRedstone"] { if raw.get(alias).is_some() { return None; } }
        let raw_tank = raw.get("tank")?;
        for alias in ["FluidName", "Amount", "Tag"] { if raw_tank.get(alias).is_some() { return None; } }
        if before.tank.to_nbt_preserving(raw_tank).as_ref() != Some(raw_tank) { return None; }
        let tank = self.tank.to_nbt_preserving(raw_tank)?;
        let Tag::List(mut slots) = raw.get("slots")?.clone() else { return None; };
        for index in 0..2 {
            let unchanged = match (&before.slots[index], &self.slots[index]) {
                (None, None) => true,
                (Some(old), Some(new)) => old.count == new.count && old.damage == new.damage && old.same_item_same_components(new),
                _ => false,
            };
            if unchanged { continue; }
            let old = before.slots[index].as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
            if slots[index] != old { return None; }
            slots[index] = self.slots[index].as_ref().map(item_to_nbt).unwrap_or_else(|| Tag::compound(Vec::<(&str, Tag)>::new()));
        }
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        fields.insert("tank".into(), tank); fields.insert("slots".into(), Tag::List(slots));
        fields.insert("redstone".into(), Tag::Byte(self.redstone as i8));
        Some(Tag::Compound(fields))
    }
