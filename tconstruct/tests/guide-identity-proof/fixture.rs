// Compile the exact production, std-only helpers in their natural module scope.
// No mock Text, extracted algorithm, Game, Font, packet or installed Mod is used.
#[path = "../../mixins/tconstruct/source-materials-book-identity.rs"]
mod identity;
#[path = "../../mixins/tconstruct/source-materials-book-index.rs"]
mod index;
