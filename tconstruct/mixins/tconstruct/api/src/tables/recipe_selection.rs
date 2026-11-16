// Ported from RecipeResult and TinkerStationBlockEntity.calcResult (147-197).
// Copyright (c) 2022 SlimeKnights. MIT; see ../../../LICENSE.
// Java revision: f4e77d43f39ba1047bf93ae816b7e1beaab514c2.

/// Keep pass distinct from a user-visible validation failure. The error payload
/// stays typed: a translated component must not be flattened into a plain string.
#[derive(Debug, PartialEq)]
pub(crate) enum RecipeResult<T, E> {
    Pass,
    Success(T),
    Failure(E),
}

impl<T, E> RecipeResult<T, E> {
    /// Java success(null) is pass, not a successful empty result object.
    pub(crate) fn success(result: Option<T>) -> Self {
        result.map_or(Self::Pass, Self::Success)
    }
}

/// Domain boundary for the station's recipe manager and synchronization. K must
/// identify a recipe instance/generation, not just its resource ID: Java compares
/// object identity when deciding whether to send the recipe again after reload.
/// No default implementations: registry lookup, validation and networking still
/// need concrete station wiring before this code can run in a game.
pub(crate) trait RecipeAccess<K, T, E> {
    fn matches(&mut self, recipe: &K) -> bool;
    fn find_matching(&mut self) -> Option<K>;
    fn validate(&mut self, recipe: &K) -> RecipeResult<T, E>;
    fn sync_recipe(&mut self, recipe: &K);
    fn sync_screen(&mut self);
}

/// Calculate in an existing level. `server` means the authoritative branch has
/// a server recipe manager. Client prediction must not search or send packets.
/// A caller replaces its previous output/error with this result; name application
/// and lazy-preview caching belong to the station, not recipe selection.
pub(crate) fn calculate_recipe<K: Clone + Eq, T, E>(last: &mut Option<K>, server: bool,
    access: &mut impl RecipeAccess<K, T, E>) -> RecipeResult<T, E> {
    if !server {
        return match last.as_ref() {
            Some(recipe) if access.matches(recipe) => access.validate(recipe),
            _ => RecipeResult::Pass,
        };
    }
    let recipe = match last.as_ref() {
        Some(recipe) if access.matches(recipe) => Some(recipe.clone()),
        _ => access.find_matching(),
    };
    let mut needs_sync = true;
    let result = match recipe {
        Some(recipe) => {
            if last.as_ref() != Some(&recipe) {
                *last = Some(recipe.clone());
                access.sync_recipe(&recipe);
                needs_sync = false;
            }
            access.validate(&recipe)
        }
        None => RecipeResult::Pass,
    };
    // Java keeps lastRecipe when no recipe matches, but still synchronizes the UI.
    if needs_sync { access.sync_screen(); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // A boundary recorder: it provides decisions, not another selection engine.
    struct Access {
        matches: bool,
        found: Option<u32>,
        validation: RecipeResult<u32, &'static str>,
        events: Vec<&'static str>,
    }
    impl RecipeAccess<u32, u32, &'static str> for Access {
        fn matches(&mut self, _: &u32) -> bool { self.events.push("matches"); self.matches }
        fn find_matching(&mut self) -> Option<u32> { self.events.push("find"); self.found }
        fn validate(&mut self, _: &u32) -> RecipeResult<u32, &'static str> {
            self.events.push("validate");
            std::mem::replace(&mut self.validation, RecipeResult::Pass)
        }
        fn sync_recipe(&mut self, _: &u32) { self.events.push("recipe"); }
        fn sync_screen(&mut self) { self.events.push("screen"); }
    }

    #[test]
    fn matching_cached_pass_or_failure_does_not_search_an_alternative() {
        for validation in [RecipeResult::Pass, RecipeResult::Failure("invalid_material")] {
            let failure = matches!(validation, RecipeResult::Failure(_));
            let mut access = Access { matches: true, found: Some(2), validation, events: vec![] };
            let mut last = Some(1);
            let result = calculate_recipe(&mut last, true, &mut access);
            assert_eq!(matches!(result, RecipeResult::Failure(_)), failure);
            assert_eq!(last, Some(1));
            assert_eq!(access.events, ["matches", "validate", "screen"]);
        }
    }

    #[test]
    fn server_syncs_changed_recipe_before_validation_and_retains_last_on_miss() {
        let mut access = Access { matches: false, found: Some(2),
            validation: RecipeResult::success(Some(4)), events: vec![] };
        let mut last = Some(1);
        assert_eq!(calculate_recipe(&mut last, true, &mut access), RecipeResult::Success(4));
        assert_eq!(last, Some(2));
        assert_eq!(access.events, ["matches", "find", "recipe", "validate"]);
        access.events.clear();
        access.found = None;
        assert_eq!(calculate_recipe(&mut last, true, &mut access), RecipeResult::Pass);
        assert_eq!(last, Some(2));
        assert_eq!(access.events, ["matches", "find", "screen"]);
    }

    #[test]
    fn client_only_validates_the_received_recipe_without_lookup_or_sync() {
        let mut access = Access { matches: true, found: Some(99),
            validation: RecipeResult::Failure("client_error"), events: vec![] };
        let mut last = Some(1);
        assert_eq!(calculate_recipe(&mut last, false, &mut access), RecipeResult::Failure("client_error"));
        assert_eq!(access.events, ["matches", "validate"]);
        access.events.clear();
        access.matches = false;
        assert_eq!(calculate_recipe(&mut last, false, &mut access), RecipeResult::Pass);
        assert_eq!(access.events, ["matches"]);
        access.events.clear();
        last = None;
        assert_eq!(calculate_recipe(&mut last, false, &mut access), RecipeResult::Pass);
        assert!(access.events.is_empty());
        assert_eq!(RecipeResult::<u32, &str>::success(None), RecipeResult::Pass);
    }
}
