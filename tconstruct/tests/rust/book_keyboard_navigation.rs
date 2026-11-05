//! Executes the production keyboard and page-turn adapters with a small Game
//! fixture. This is not a real host window/input/LAN test; CI checks host types.
#![allow(dead_code)]
use std::marker::PhantomData;
use std::ops::ControlFlow;

#[derive(Clone, Copy, Debug)]
enum KeyCode { ArrowLeft, ArrowRight, KeyA, KeyD, Escape, PageUp, PageDown, ArrowUp, KeyW }
struct Style { insertion: Option<String> }
struct Span { style: Style }
struct Text { spans: Vec<Span> }
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
struct Game<'a> { book: Option<BookScreen>, world: World, audio: Audio, marker: PhantomData<&'a ()> }
impl Game<'_> {
    fn turn_page(&mut self, forward: bool) { assert!(tconstruct_materials_book_turn(self, forward).is_break()); }
    fn page(&self) -> usize { match self.book.as_ref().unwrap() { BookScreen::View { page, .. } => *page, _ => panic!("not a view") } }
}
#[path = "../../mixins/tconstruct/source-materials-book-spread.rs"] mod tconstruct_materials_book_spread;
include!("../../mixins/tconstruct/source-materials-book-key.rs");
include!("../../mixins/tconstruct/source-materials-book-turn.rs");

fn guide(count: usize, page: usize) -> Game<'static> {
    let pages = (0..count).map(|i| Text { spans: vec![Span { style: Style { insertion: (i == 0).then(|| "tconstruct:materials-book-root".into()) } }] }).collect();
    Game { book: Some(BookScreen::View { pages, page, lectern: None }), world: World::default(), audio: Audio::default(), marker: PhantomData }
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
fn escape_page_keys_text_and_unrelated_keys_fall_through_without_mutation() {
    let mut game = guide(6, 2);
    game.world.chunks.history = Some(0);
    for key in [None, Some(KeyCode::Escape), Some(KeyCode::PageUp), Some(KeyCode::PageDown), Some(KeyCode::ArrowUp), Some(KeyCode::KeyW)] {
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
        for key in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::KeyA, KeyCode::KeyD] {
            assert!(tconstruct_materials_book_key(&mut game, Some(key)).is_continue());
        }
        assert_eq!(game.world.chunks.history, Some(0));
        assert_eq!(game.audio.turns, 0);
        if let Some(BookScreen::View { page, .. }) = game.book { assert_eq!(page, 2); }
    }
}
