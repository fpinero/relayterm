//! Application boundary for provider-neutral use cases.
//!
//! Future services coordinate domain transitions through persistence, clock,
//! identifier, event, supervision, and Git ports. Introduce ports only when a
//! use case needs them. This crate must not depend on concrete adapters or UI.
//! M01 deliberately implements no placeholder repositories or fake mutations.
