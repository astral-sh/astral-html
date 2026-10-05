//! An HTML parser.
//!
//! [`Reader`] yields tokens in source order and selects HTML text states.
//! [`Tokenizer`] gives callers explicit control over those states. [`Document`]
//! retains lexical element scopes for repeated queries, with configurable [`Limits`].
//!
//! Input must already be decoded UTF-8. Unchanged strings borrow from that input.
//! The parser does not implement browser tree construction.

mod document;
mod entities;
mod entities_data;
mod reader;
mod tokenizer;

pub use document::{Document, Element, Error, Limits};
pub use entities::{decode, normalize};
pub use reader::Reader;
pub use tokenizer::{Attribute, Doctype, State, Tag, Token, Tokenizer};
