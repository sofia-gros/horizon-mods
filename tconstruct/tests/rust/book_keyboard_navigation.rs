//! Executes the production keyboard and page-turn adapters with a small Game
//! fixture. No renderer/input/LAN execution or official host type proof is claimed.
#![allow(dead_code)]
use std::marker::PhantomData;
use std::ops::ControlFlow;

#[derive(Clone, Copy, Debug)]
enum KeyCode { ArrowLeft, ArrowRight, KeyA, KeyD, Escape, PageUp, PageDown, ArrowUp, KeyW }
#[derive(Clone)]
struct Style { insertion: Option<String>, click: Option<crate::text::ClickEvent> }
#[derive(Clone)]
struct Span { style: Style }
#[derive(Clone)]
struct Text { spans: Vec<Span> }
impl Text {
    fn resolve_legacy(&self) -> Self { self.clone() }
    fn chars(&self) -> std::iter::Empty<(char, &Style)> { std::iter::empty() }
}
struct Font;
impl Font { fn split(&self, text: &Text, _: f32) -> Vec<Text> { vec![text.clone()] } }
struct Atlas { font: Font }
struct Gui { atlas: Atlas }
struct HudBatch { width: f32, height: f32 }
impl HudBatch { fn new(size: [u32; 2]) -> Self { Self { width: size[0] as f32, height: size[1] as f32 } } }
enum BookScreen { View { pages: Vec<Text>, page: usize, lectern: Option<(i32, i32, i32)> }, Edit }
#[derive(Default)]
struct Chunks { history: Option<usize> }
impl Chunks {
    fn __horizon_mixins_tconstruct_materials_book_history_mut(&mut self) -> &mut Option<usize> { &mut self.history }
}
#[derive(Default)]
struct World { chunks: Chunks }
#[derive(Default)]
struct Audio { turns: usize }
impl Audio {
    fn play(&mut self, id: &str, pos: Option<[f64; 3]>, volume: f32, pitch: f32) {
        assert_eq!(id, "minecraft:item.book.page_turn");
        assert_eq!((pos, volume, pitch), (None, 1.0, 1.0));
        self.turns += 1;
    }
}
struct Game<'a> { book: Option<BookScreen>, world: World, audio: Audio, marker: PhantomData<&'a ()>, cursor: (f32, f32), gui: Gui }
impl Game<'_> {
    // There is intentionally no turn_page method: production guide helpers
    // must not route through or replace PlutoLib's native one-page operation.
    fn book_follow_link(&mut self, _: crate::text::ClickEvent, _: usize, _: Option<(i32, i32, i32)>) { panic!("link execution is outside this navigation fixture"); }
    fn page(&self) -> usize { match self.book.as_ref().unwrap() { BookScreen::View { page, .. } => *page, _ => panic!("not a view") } }
}
#[path = "../../mixins/tconstruct/source-pure-rules.rs"] mod tconstruct_materials_book_spread;
include!("../../mixins/tconstruct/source-materials-book-key.rs");
#[path = "../../mixins/tconstruct/source-materials-book-navigation.rs"] mod tconstruct_materials_book_navigation;
include!("../../mixins/tconstruct/source-materials-book-control.rs");
include!("../../mixins/tconstruct/source-materials-book-click.rs");

fn guide(count: usize, page: usize) -> Game<'static> {
    let pages = (0..count).map(|i| Text { spans: vec![Span { style: Style { insertion: (i == 0).then(|| "tconstruct:materials-book-root".into()), click: None } }] }).collect();
    Game { book: Some(BookScreen::View { pages, page, lectern: None }), world: World::default(), audio: Audio::default(), marker: PhantomData, cursor: (0.0, 0.0), gui: Gui { atlas: Atlas { font: Font } } }
}

#[test]
fn arrows_and_ad_move_real_spreads_and_clear_link_return() {
    let mut game = guide(5, usize::MAX);
    for (key, target) in [(KeyCode::ArrowRight, 0), (KeyCode::KeyD, 2), (KeyCode::KeyD, 4), (KeyCode::KeyA, 2), (KeyCode::ArrowLeft, 0), (KeyCode::KeyA, usize::MAX)] {
        game.world.chunks.history = Some(2);
        assert!(tconstruct_materials_book_key(&mut game, Some(key)).is_break());
        assert_eq!(game.page(), target);
        assert_eq!(game.world.chunks.history, None);
    }
    assert_eq!(game.audio.turns, 6);
}

#[test]
fn repeated_keys_respect_cover_and_last_odd_or_even_spread() {
    for count in [1, 2, 5, 6] {
        let mut game = guide(count, usize::MAX);
        for _ in 0..10 { assert!(tconstruct_materials_book_key(&mut game, Some(KeyCode::ArrowRight)).is_break()); }
        assert_eq!(game.page(), ((count - 1) / 2) * 2);
        for _ in 0..10 { assert!(tconstruct_materials_book_key(&mut game, Some(KeyCode::ArrowLeft)).is_break()); }
        assert_eq!(game.page(), usize::MAX);
    }
}

#[test]
fn escape_text_and_unrelated_keys_fall_through_without_mutation() {
    let mut game = guide(6, 2);
    game.world.chunks.history = Some(0);
    for key in [None, Some(KeyCode::Escape), Some(KeyCode::ArrowUp), Some(KeyCode::KeyW)] {
        assert_eq!(tconstruct_materials_book_key(&mut game, key), ControlFlow::Continue(()));
        assert_eq!(game.page(), 2);
        assert_eq!(game.world.chunks.history, Some(0));
        assert_eq!(game.audio.turns, 0);
    }
}

#[test]
fn vanilla_views_edit_lectern_closed_and_empty_views_are_untouched() {
    for kind in 0..7 {
        let mut game = guide(5, 2);
        match kind {
            0 => if let Some(BookScreen::View { pages, .. }) = &mut game.book { pages[0].spans[0].style.insertion = None; },
            1 => game.book = Some(BookScreen::Edit),
            2 => if let Some(BookScreen::View { lectern, .. }) = &mut game.book { *lectern = Some((0, 64, 0)); },
            3 => game.book = None,
            4 => if let Some(BookScreen::View { pages, .. }) = &mut game.book { pages.clear(); },
            5 => if let Some(BookScreen::View { pages, .. }) = &mut game.book { pages[0].spans.clear(); },
            _ => if let Some(BookScreen::View { pages, .. }) = &mut game.book { pages[0].spans[0].style.insertion = Some("tconstruct:book-title".into()); },
        }
        game.world.chunks.history = Some(0);
        for key in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::KeyA, KeyCode::KeyD, KeyCode::PageUp, KeyCode::PageDown] {
            assert!(tconstruct_materials_book_key(&mut game, Some(key)).is_continue());
        }
        assert_eq!(game.world.chunks.history, Some(0));
        assert_eq!(game.audio.turns, 0);
        if let Some(BookScreen::View { page, .. }) = game.book { assert_eq!(page, 2); }
    }
}

#[test]
fn native_page_keys_keep_spread_policy_and_audio_exactly_once() {
    let mut game = guide(5, usize::MAX);
    for (key, target) in [(KeyCode::PageDown, 0), (KeyCode::PageDown, 2), (KeyCode::PageDown, 4), (KeyCode::PageDown, 4), (KeyCode::PageUp, 2)] {
        let before = game.audio.turns;
        assert!(tconstruct_materials_book_key(&mut game, Some(key)).is_break());
        assert_eq!(game.page(), target);
        assert_eq!(game.audio.turns, before + 1);
    }
}

#[test]
fn control_next_back_reports_native_shape_without_reentering_native_turn() {
    let mut game = guide(5, usize::MAX);
    for (action, target) in [("next", 0), ("next", 2), ("back", 0), ("back", usize::MAX), ("back", usize::MAX)] {
        game.world.chunks.history = Some(2);
        let ControlFlow::Break(Ok(value)) = tconstruct_materials_book_control(&mut game, Some(action)) else { panic!("guide command not consumed"); };
        assert_eq!(value, serde_json::json!({"page":target,"pages":5,"lectern":null}));
        assert_eq!(game.page(), target);
        assert_eq!(game.world.chunks.history, None);
    }
    assert_eq!(game.audio.turns, 5);
}

#[test]
fn control_queries_other_actions_and_non_guides_keep_native_authority() {
    let mut game = guide(5, usize::MAX);
    for action in [None, Some("done"), Some("sign"), Some("take"), Some("unknown")] {
        assert!(tconstruct_materials_book_control(&mut game, action).is_continue());
        assert_eq!(game.page(), usize::MAX);
        assert_eq!(game.audio.turns, 0);
    }
    for book in [None, Some(BookScreen::Edit), Some(BookScreen::View { pages: vec![], page: 0, lectern: None }), Some(BookScreen::View { pages: vec![], page: 0, lectern: Some((0, 64, 0)) })] {
        game.book = book;
        assert!(tconstruct_materials_book_control(&mut game, Some("next")).is_continue());
        assert!(tconstruct_materials_book_control(&mut game, Some("back")).is_continue());
        assert_eq!(game.audio.turns, 0);
    }
}

#[test]
fn click_arrows_preserve_cover_spread_history_and_single_sound() {
    let mut game = guide(5, usize::MAX);
    // Batch 800x600 => center400/top200; no text-row hit in these controls.
    for (cursor, target) in [((481.0, 376.0), 0), ((566.0, 376.0), 2), ((217.0, 376.0), 0), ((217.0, 376.0), usize::MAX)] {
        game.cursor = cursor;
        game.world.chunks.history = Some(2);
        let before = game.audio.turns;
        assert!(tconstruct_materials_book_click(&mut game, [800,600]).is_break());
        assert_eq!(game.page(), target);
        assert_eq!(game.world.chunks.history, None);
        assert_eq!(game.audio.turns, before + 1);
    }
}
