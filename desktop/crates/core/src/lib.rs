//! tandem_core: framework-free domain for the desktop — call mirror model,
//! controller state machine, reconciliation, and emergency pre-check. Everything
//! here is deterministic and unit-testable with no I/O (docs/14 layering).

pub mod controller;
pub mod emergency;
pub mod error;
pub mod events;
pub mod model;
pub mod reconcile;

pub use controller::CallController;
pub use error::CoreError;
pub use model::{Call, CallSnapshot, StateVersion};
