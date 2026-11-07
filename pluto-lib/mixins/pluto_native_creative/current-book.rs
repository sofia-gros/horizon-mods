// Current stored-book adapter. No registry, owner token, state extension or
// automatic input interception. Installed inside game::books for private APIs.
use super::{BookScreen, Game};
use horizon_client::{pluto_guide_text, pluto_inventory::guide::current_book as api};

fn context_available(game: &Game<'_>) -> bool {
    // Conservative local-gameplay admission, not a physical-input event token.
    game.options.execution_role.is_client()
        && game.lan_acting.is_none()
        && game.player.is_some()
        && game.renderer.is_some()
        && game.title.is_none()
        && game.screen == crate::game::Screen::Playing
        && game.sign_edit.is_none()
        && !game.dialog_open()
        && game.command_prompt.is_none()
        && game.link_prompt.is_none()
        && game.code_of_conduct.is_none()
        && game.server_packs.prompt.is_none()
        && game.pending_entry.is_none()
        && game.integrated_starting.is_none()
        && game.pending_save.is_none()
        && game.pack_reload.is_none()
        // remote_loading() alone deliberately excludes some portal loads.
        && game.remote.as_ref().is_none_or(|remote| {
            !remote.level_load.is_active() && remote.reconfiguring.is_none()
        })
}
fn capture_book(book: Option<&BookScreen>) -> Result<api::Snapshot, api::Unavailable> {
    let Some(book) = book else {
        return Err(api::Unavailable::NoBook);
    };
    match book {
        BookScreen::Edit { .. } => Err(api::Unavailable::NotReadOnlyView),
        BookScreen::View {
            lectern: Some(_), ..
        } => Err(api::Unavailable::Lectern),
        BookScreen::View {
            pages,
            page,
            lectern: None,
        } => {
            // Metadata is opaque inspection data, never interpreted as authority.
            let document = pluto_guide_text::from_native(pages, |_| true)
                .map_err(api::Unavailable::Document)?;
            api::Snapshot::new(document, *page)
        }
    }
}
fn capture(game: &Game<'_>) -> Result<api::Snapshot, api::Unavailable> {
    if !context_available(game) {
        return Err(api::Unavailable::Context);
    }
    capture_book(game.book.as_ref())
}
struct Current<'a, 'g, 's> {
    game: &'a mut Game<'g>,
    entry: &'s api::Snapshot,
    invoked: bool,
}
impl api::PageTurn for Current<'_, '_, '_> {
    fn turn(
        &mut self,
        expected: &api::Snapshot,
        direction: api::Direction,
    ) -> Result<api::TurnOutcome, api::TurnError> {
        if self.invoked {
            return Err(api::TurnError::AlreadyInvoked);
        }
        let current = capture(self.game).map_err(api::TurnError::Unavailable)?;
        let after = api::check_turn(self.entry, &current, expected, direction)?;
        self.invoked = true;
        // Call the original route, including the sound at an unchanged edge.
        // Required whole-plan guard excludes foreign turn_page hooks.
        self.game.turn_page(direction == api::Direction::Forward);
        let actual = capture(self.game).map_err(api::TurnError::OutcomeUnavailable)?;
        if actual.document() != expected.document() || actual.page() != after {
            return Err(api::TurnError::OutcomeChanged);
        }
        Ok(api::TurnOutcome {
            before: expected.page(),
            after: actual.page(),
        })
    }
}
/// Borrow current facts and at most one native page turn for this call only.
/// This does not attest input ownership or distinguish identical reopenings.
/// Caller controls timing; using this during a native input handler does not
/// automatically consume that input. Do not also let it perform the same turn.
pub(crate) fn with_current<R>(
    game: &mut Game<'_>,
    run: impl FnOnce(&api::Snapshot, &mut dyn api::PageTurn) -> R,
) -> Result<R, api::Unavailable> {
    let entry = capture(game)?;
    let mut current = Current {
        game,
        entry: &entry,
        invoked: false,
    };
    Ok(run(&entry, &mut current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::text::{ClickEvent, Text};
    #[test]
    fn real_native_read_only_view_projects_bounded_semantics_and_page() {
        let native = BookScreen::View {
            pages: vec![Text::literal("first"), Text::empty()],
            page: 1,
            lectern: None,
        };
        let snapshot = capture_book(Some(&native)).unwrap();
        assert_eq!(snapshot.page(), 1);
        assert_eq!(snapshot.document().pages()[0][0].text, "first");
        assert!(snapshot.document().pages()[1].is_empty());
        assert!(matches!(
            native,
            BookScreen::View {
                page: 1,
                lectern: None,
                ..
            }
        ));
    }
    #[test]
    fn real_native_edit_lectern_and_invalid_page_are_not_reinterpreted() {
        assert_eq!(capture_book(None), Err(api::Unavailable::NoBook));
        let edit = BookScreen::Edit {
            slot: 0,
            pages: vec![],
            page: 0,
            signing: false,
            title: String::new(),
            title_edit: super::super::book_title_box(),
            page_edit: super::super::book_page_box(),
        };
        assert_eq!(
            capture_book(Some(&edit)),
            Err(api::Unavailable::NotReadOnlyView)
        );
        let lectern = BookScreen::View {
            pages: vec![Text::literal("text")],
            page: 0,
            lectern: Some((1, 2, 3)),
        };
        assert_eq!(capture_book(Some(&lectern)), Err(api::Unavailable::Lectern));
        let cover = BookScreen::View {
            pages: vec![Text::literal("text")],
            page: usize::MAX,
            lectern: None,
        };
        assert_eq!(
            capture_book(Some(&cover)),
            Err(api::Unavailable::InvalidPage)
        );
    }
    #[test]
    fn unsupported_native_actions_fail_instead_of_losing_them() {
        let mut page = Text::literal("link");
        page.spans[0].style.click = Some(ClickEvent::OpenUrl("https://example.test".into()));
        let native = BookScreen::View {
            pages: vec![page],
            page: 0,
            lectern: None,
        };
        assert_eq!(
            capture_book(Some(&native)),
            Err(api::Unavailable::Document(
                horizon_client::pluto_inventory::guide::document::Error::UnsupportedStyle
            ))
        );
    }
}
