//! A read-only HTML parser for package indexes.
//!
//! This crate provides HTML tokenization and a source-order document view.
//! It does not implement browser tree construction or execute scripts.

mod entities;
mod entities_data;
mod reader;
mod tokenizer;

pub use entities::{decode, normalize};
pub use reader::Reader;
pub use tokenizer::{Attribute, Doctype, State, Tag, Token, Tokenizer};
