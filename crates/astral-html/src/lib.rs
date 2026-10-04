//! A read-only HTML parser for package indexes.
//!
//! This crate provides HTML tokenization and a source-order document view.
//! It does not implement browser tree construction or execute scripts.

mod entities;
mod entities_data;

pub use entities::{decode, normalize};
