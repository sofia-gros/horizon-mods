// Consumer-specific book facts. These values are not a screen owner, request
// token or permission to trust received metadata. Legacy title routing remains
// explicitly legacy; this refactor does not enable translated book identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BookId {
    Materials,
    Puny,
    Mighty,
    Encyclopedia,
    Gadgetry,
    Foundry,
}

impl BookId {
    pub(crate) fn from_item(item: &str) -> Option<Self> {
        match item {
            "tconstruct:materials_and_you" => Some(Self::Materials),
            "tconstruct:puny_smelting" => Some(Self::Puny),
            "tconstruct:mighty_smelting" => Some(Self::Mighty),
            "tconstruct:encyclopedia" => Some(Self::Encyclopedia),
            "tconstruct:tinkers_gadgetry" => Some(Self::Gadgetry),
            "tconstruct:fantastic_foundry" => Some(Self::Foundry),
            _ => None,
        }
    }

    pub(crate) const fn source_parts(self) -> (&'static str, &'static str) {
        match self {
            Self::Materials => ("assets/tconstruct/book/materials_and_you/", "Materials & You"),
            Self::Puny => ("assets/tconstruct/book/puny_smelting/", "Puny Smelting"),
            Self::Mighty => ("assets/tconstruct/book/mighty_smelting/", "Mighty Smelting"),
            Self::Encyclopedia => ("assets/tconstruct/book/encyclopedia/", "Encyclopedia"),
            Self::Gadgetry => ("assets/tconstruct/book/tinkers_gadgetry/", "Tinkers' Gadgetry"),
            Self::Foundry => ("assets/tconstruct/book/fantastic_foundry/", "Fantastic Foundry"),
        }
    }

    const fn wire_name(self) -> &'static str {
        match self {
            Self::Materials => "materials",
            Self::Puny => "puny",
            Self::Mighty => "mighty",
            Self::Encyclopedia => "encyclopedia",
            Self::Gadgetry => "gadgetry",
            Self::Foundry => "foundry",
        }
    }

    const fn sprites(self) -> (&'static str, &'static str) {
        match self {
            Self::Materials => ("gui/tconstruct_book_covers_materials_and_you_cover", "gui/tconstruct_book_covers_materials_and_you_pages"),
            Self::Puny => ("gui/tconstruct_book_covers_puny_smelting_cover", "gui/tconstruct_book_covers_puny_smelting_pages"),
            Self::Mighty => ("gui/tconstruct_book_covers_mighty_smelting_cover", "gui/tconstruct_book_covers_mighty_smelting_pages"),
            Self::Encyclopedia => ("gui/tconstruct_book_covers_encyclopedia_cover", "gui/tconstruct_book_covers_encyclopedia_pages"),
            Self::Gadgetry => ("gui/tconstruct_book_covers_gadgetry_cover", "gui/tconstruct_book_covers_gadgetry_pages"),
            Self::Foundry => ("gui/tconstruct_book_covers_fantastic_foundry_cover", "gui/tconstruct_book_covers_fantastic_foundry_pages"),
        }
    }
}

fn legacy_title(title: &str) -> Option<BookId> {
    [BookId::Materials, BookId::Puny, BookId::Mighty, BookId::Encyclopedia, BookId::Gadgetry, BookId::Foundry]
        .into_iter()
        .find(|id| title.strip_prefix(id.source_parts().1).is_some_and(|tail| tail.starts_with('\n')))
}

/// The wire caller must first perform its unchanged root/first-span checks.
/// Unknown titles reject. This label is not serialized as a new wire field.
pub(crate) fn legacy_wire_id(title: &str) -> Option<&'static str> {
    legacy_title(title).map(BookId::wire_name)
}

/// The draw caller must first perform its unchanged root/first-span checks.
/// Unlike wire admission, unknown titles retain the Materials sprite fallback.
pub(crate) fn legacy_draw_sprites(title: &str) -> (&'static str, &'static str) {
    legacy_title(title).unwrap_or(BookId::Materials).sprites()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent legacy predicates, matching the pre-refactor branches.
    fn old_wire(title: &str) -> Option<&'static str> {
        if title.starts_with("Materials & You\n") { Some("materials") }
        else if title.starts_with("Puny Smelting\n") { Some("puny") }
        else if title.starts_with("Mighty Smelting\n") { Some("mighty") }
        else if title.starts_with("Encyclopedia\n") { Some("encyclopedia") }
        else if title.starts_with("Tinkers' Gadgetry\n") { Some("gadgetry") }
        else if title.starts_with("Fantastic Foundry\n") { Some("foundry") }
        else { None }
    }
    fn old_draw(title: &str) -> (&'static str, &'static str) {
        if title.starts_with("Tinkers' Gadgetry\n") { ("gui/tconstruct_book_covers_gadgetry_cover", "gui/tconstruct_book_covers_gadgetry_pages") }
        else if title.starts_with("Fantastic Foundry\n") { ("gui/tconstruct_book_covers_fantastic_foundry_cover", "gui/tconstruct_book_covers_fantastic_foundry_pages") }
        else if title.starts_with("Puny Smelting\n") { ("gui/tconstruct_book_covers_puny_smelting_cover", "gui/tconstruct_book_covers_puny_smelting_pages") }
        else if title.starts_with("Mighty Smelting\n") { ("gui/tconstruct_book_covers_mighty_smelting_cover", "gui/tconstruct_book_covers_mighty_smelting_pages") }
        else if title.starts_with("Encyclopedia\n") { ("gui/tconstruct_book_covers_encyclopedia_cover", "gui/tconstruct_book_covers_encyclopedia_pages") }
        else { ("gui/tconstruct_book_covers_materials_and_you_cover", "gui/tconstruct_book_covers_materials_and_you_pages") }
    }
    const CASES: [(&str, BookId, &str, &str); 6] = [
        ("tconstruct:materials_and_you", BookId::Materials, "materials_and_you", "Materials & You"),
        ("tconstruct:puny_smelting", BookId::Puny, "puny_smelting", "Puny Smelting"),
        ("tconstruct:mighty_smelting", BookId::Mighty, "mighty_smelting", "Mighty Smelting"),
        ("tconstruct:encyclopedia", BookId::Encyclopedia, "encyclopedia", "Encyclopedia"),
        ("tconstruct:tinkers_gadgetry", BookId::Gadgetry, "tinkers_gadgetry", "Tinkers' Gadgetry"),
        ("tconstruct:fantastic_foundry", BookId::Foundry, "fantastic_foundry", "Fantastic Foundry"),
    ];

    #[test]
    fn source_item_root_and_title_pairs_are_unchanged() {
        for (item, id, directory, title) in CASES {
            assert_eq!(BookId::from_item(item), Some(id));
            assert_eq!(id.source_parts(), (format!("assets/tconstruct/book/{directory}/").as_str(), title));
        }
    }
    #[test]
    fn six_legacy_titles_and_body_suffixes_match_the_old_branches() {
        for (_, _, _, title) in CASES {
            for suffix in ["\n", "\n\nContents", "\n日本語", "\n\0"] {
                let title = format!("{title}{suffix}");
                assert_eq!(legacy_wire_id(&title), old_wire(&title));
                assert_eq!(legacy_draw_sprites(&title), old_draw(&title));
            }
        }
    }
    #[test]
    fn near_titles_do_not_normalize_case_space_or_newlines() {
        for (_, _, _, title) in CASES {
            for wrong in [title.to_owned(), format!("{title}\r\n"), format!(" {title}\n"), format!("{}\n", title.to_lowercase()), format!("{title} extra\n")] {
                assert_eq!(legacy_wire_id(&wrong), old_wire(&wrong));
                assert_eq!(legacy_draw_sprites(&wrong), old_draw(&wrong));
            }
        }
    }
    #[test]
    fn unknown_and_translated_titles_preserve_the_asymmetric_fallback() {
        for title in ["", "Unknown\n", "道具とあなた\n", "Materials & You", "\nPuny Smelting\n"] {
            assert_eq!(legacy_wire_id(title), None);
            assert_eq!(legacy_draw_sprites(title), BookId::Materials.sprites());
        }
    }
    #[test]
    fn item_ids_are_exact_and_not_translated_titles_or_prefixes() {
        for item in ["", "materials_and_you", "Materials & You", "tconstruct:materials_and_you_extra", "other:materials_and_you", "TCONSTRUCT:materials_and_you"] {
            assert_eq!(BookId::from_item(item), None);
        }
    }
}
