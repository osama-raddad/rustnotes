//! The Vynatix component library, one module per component. Every
//! component wraps the shared foundations ([`crate::states`],
//! [`crate::theme`]) so the focus ring, the state overlays and the kit
//! flags behave the same everywhere.

pub mod focus_ring;

pub use focus_ring::FocusRing;
