// Reuse real host dependencies and production modules from the existing fixture.
// Its potion-name/capacity stubs are not called by the survival-chain cases.
include!("source_seared_reinforcement.rs");
#[cfg(test)]
#[path = "survival_chain_cases.rs"]
mod survival_chain_tests;
