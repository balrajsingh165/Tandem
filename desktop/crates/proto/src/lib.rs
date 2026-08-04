//! tandem_proto: re-exports the prost-generated tandem.v1 types as the single
//! Rust wire-type surface. No hand-written types beyond include! glue.

pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/tandem.v1.rs"));
}

pub use v1::*;
