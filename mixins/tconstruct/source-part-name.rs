// Source MaterialItem.getName and MaterialTooltipCache key resolution. The
// caller supplies translations from the active client resource stack.
pub(super) fn source_part_name(
    item: &str,
    material: &str,
    translate: &dyn Fn(&str) -> Option<String>,
    format: &dyn Fn(&str, &[String]) -> String,
) -> Option<String> {
    let (item_namespace, item_path) = item.split_once(':')?;
    let item_key = format!("item.{item_namespace}.{item_path}");
    let part_name = translate(&item_key).unwrap_or_else(|| item_key.clone());
    let (base, variant) = material.split_once('#').unwrap_or((material, ""));
    let (namespace, path) = base.split_once(':').unwrap_or(("minecraft", base));
    let namespace = if namespace.is_empty() { "minecraft" } else { namespace };
    if path.is_empty() || !namespace.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        || !path.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
        || !variant.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c)) {
        return Some(part_name);
    }
    if namespace == "tconstruct" && path == "unknown" && variant.is_empty() { return Some(part_name); }
    let try_name = |location: &str| {
        if let Some(name) = translate(&format!("{item_key}.{namespace}.{location}")) { return Some(name); }
        let material_key = format!("material.{namespace}.{location}");
        if let Some(template) = translate(&format!("{material_key}.format")) {
            return Some(format(&template, std::slice::from_ref(&part_name)));
        }
        if let Some(material_name) = translate(&material_key) {
            let template = translate("item.tconstruct.tool.format").unwrap_or_else(|| "item.tconstruct.tool.format".into());
            return Some(format(&template, &[material_name, part_name.clone()]));
        }
        None
    };
    if !variant.is_empty() {
        if let Some(name) = try_name(&format!("{path}.{variant}")) { return Some(name); }
    }
    Some(try_name(path).unwrap_or(part_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_english_material_names() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/assets/tconstruct/lang/en_us.json"))
            .find(|p| p.is_file()).expect("pinned English translations");
        let values: std::collections::HashMap<String, String> =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let translate = |key: &str| values.get(key).cloned();
        let format = |template: &str, args: &[String]| args.iter().fold(template.to_owned(), |s, a| s.replacen("%s", a, 1));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:flint", &translate, &format).as_deref(), Some("Flint Pick Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:wood", &translate, &format).as_deref(), Some("Wooden Pick Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:copper#oxidized", &translate, &format).as_deref(), Some("Oxidized Copper Pick Head"));
    }

    #[test]
    fn pinned_japanese_uses_english_fallback() {
        use std::io::Read;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors()
            .map(|p| p.join("content/reference/tconstruct-part-visuals.zip"))
            .find(|p| p.is_file()).expect("pinned translations archive");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut values = std::collections::HashMap::<String, String>::new();
        for code in ["en_us", "ja_jp"] {
            let mut data = Vec::new();
            zip.by_name(&format!("assets/tconstruct/lang/{code}.json")).unwrap().read_to_end(&mut data).unwrap();
            values.extend(serde_json::from_slice::<std::collections::HashMap<String, String>>(&data).unwrap());
        }
        let translate = |key: &str| values.get(key).cloned();
        let format = |template: &str, args: &[String]| args.iter().fold(template.to_owned(), |s, a| s.replacen("%s", a, 1));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:wood", &translate, &format).as_deref(), Some("木のツルハシ頭"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:flint", &translate, &format).as_deref(), Some("火打石のツルハシ頭"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:copper#oxidized", &translate, &format).as_deref(), Some("酸化した銅のツルハシ頭"));
    }

    #[test]
    fn original_priority_variant_then_base_and_unknown() {
        let values = [
            ("item.tconstruct.pick_head", "Pick Head"),
            ("item.tconstruct.pick_head.tconstruct.copper.oxidized", "Specific Oxidized Head"),
            ("material.tconstruct.copper", "Copper"),
            ("material.tconstruct.wood.format", "Wooden %s"),
            ("item.tconstruct.tool.format", "%s %s"),
        ];
        let translate = |key: &str| values.iter().find(|(k,_)| *k == key).map(|(_,v)| (*v).to_owned());
        let format = |template: &str, args: &[String]| args.iter().fold(template.to_owned(), |s, a| s.replacen("%s", a, 1));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:copper#oxidized", &translate, &format).as_deref(), Some("Specific Oxidized Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:copper#other", &translate, &format).as_deref(), Some("Copper Pick Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:wood", &translate, &format).as_deref(), Some("Wooden Pick Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:unknown", &translate, &format).as_deref(), Some("Pick Head"));
        assert_eq!(source_part_name("tconstruct:pick_head", "tconstruct:wood#Bad", &translate, &format).as_deref(), Some("Pick Head"));
    }
}
