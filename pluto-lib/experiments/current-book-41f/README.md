# Current-book source adaptation to official 41f

This candidate changes one ordinary metadata JSON, not the adapter, guard,
native source or TConstruct behavior. It updates only official revisions and
whole-file hashes for books.rs, game.rs and remote.rs. execution_role.rs is
byte-identical. Reversing those metadata fields restores the prior JSON bytes.

Complete plan run42231588813 accepted integration9cc1 on official41f/5a64.
The unchanged current-book guard rejected its first official-source input,
books.rs. Its earlier exact module placement/source and no-turn-hook checks
therefore ran without rejecting. The full raw plan was not retained, so this
does not supply native node identities or a complete guard pass. The separate
payload guard stopped earlier at native observer location, which this candidate
does not guess, update or bypass. The existing LAN cancellation conflict remains
for the private bridge and later actual plan to address.

The complete old/new books.rs differs only in draw_book: one width binding and
the background fill's delegation to render_transparent_background. Exact reverse
substitution restores all624c bytes. BookScreen, turn_page (including audio),
book input and all other book methods are unchanged. The current-book adapter
does not replace drawing. This finding does not repair TConstruct's separate
custom draw path or prove its new native background-hook integration.

Game and remote whole files have other native changes. No old bodies are copied
over those changes. The existing adapter still requires Client, a renderer,
an attached player, no guest, Screen::Playing and the listed prompt/loading
exclusions. New EmoteWheel/HudEditor screen variants do not equal Playing. This
source check is not a live input-admission or retained-screen ownership proof.

Eight pure source/inverse and synthetic guard regressions passed. Set
GUIDE_BASE_ROOT to the matching prior Mod source, GUIDE_624C_ROOT to pristine
624c, GUIDE_41F_ROOT to the pinned partial41f source root, and GUIDE_41F_GAME to
the exact41f game.rs. They do not call rustfmt, Cargo, planner or native code.
Synthetic module/hook rows are not actual plan IR. The ordinary full guard must
run again on the complete plan, followed by applied host typing. Client/UI/audio,
language reload, real replies and LAN remain separate unproved boundaries.
