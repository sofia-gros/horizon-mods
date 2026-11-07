//! Host-independent PlutoLib types: no dependency on Horizon, so they build and test in seconds.
//! (`pluto-lib/tests/base` compiles this module on its own.)

pub mod action_result;
pub mod event;
pub mod identifier;
pub mod support;

pub use action_result::ActionResult;
pub use event::{Callback, Combiner, Event, PhaseOrderingCycle, default_phase};
pub use identifier::{DEFAULT_NAMESPACE, Identifier, IdentifierError};
pub use support::{Support, SupportEntry};
