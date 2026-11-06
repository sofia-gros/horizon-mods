    pub(super) fn to_nbt_preserving(&self, raw: &Tag) -> Option<Tag> {
        let before = Self::from_nbt(raw)?;
        let canonical = self.to_nbt();
        if Self::from_nbt(&canonical).as_ref() != Some(self) { return None; }
        let Tag::Compound(mut fields) = raw.clone() else { return None; };
        for alias in ["Capacity", "Fluids", "Fluid", "Tank", "FluidName", "Amount", "Tag"] {
            if raw.get(alias).is_some() { return None; }
        }
        let Tag::List(old_rows) = raw.get("fluids")? else { return None; };
        let old_canonical = before.to_nbt();
        let Tag::List(old_normal) = old_canonical.get("fluids")? else { return None; };
        let Tag::List(new_normal) = canonical.get("fluids")? else { return None; };
        for (index, fluid) in before.fluids.iter().enumerate() {
            let row = &old_rows[index];
            let Tag::String(id) = row.get("id")? else { return None; };
            if id.to_string().ok()?.as_str() != fluid.id.as_str() { return None; }
            for alias in ["FluidName", "Amount", "Tag", "fluid"] { if row.get(alias).is_some() { return None; } }
            if !self.fluids.iter().any(|next| next.id == fluid.id && next.tag == fluid.tag)
                && row != &old_normal[index] { return None; }
        }
        let mut rows = Vec::with_capacity(self.fluids.len());
        for (index, fluid) in self.fluids.iter().enumerate() {
            if let Some(old) = before.fluids.iter().position(|old| old.id == fluid.id && old.tag == fluid.tag) {
                let Tag::Compound(mut row) = old_rows[old].clone() else { return None; };
                row.insert("amount".into(), Tag::Long(i64::from(fluid.amount)));
                rows.push(Tag::Compound(row));
            } else { rows.push(new_normal[index].clone()); }
        }
        fields.insert("capacity".into(), Tag::Long(i64::from(self.capacity)));
        fields.insert("fluids".into(), Tag::List(rows));
        Some(Tag::Compound(fields))
    }
