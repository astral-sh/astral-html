//! A high-performance HTML parser designed for document traversal.
//!
//! [`Reader`] yields tokens in source order and selects HTML text states.
//! [`Tokenizer`] gives callers explicit control over those states. [`Document`]
//! retains lexical element scopes for repeated queries, with configurable [`Limits`].
//!
//! Input must already be decoded UTF-8. Unchanged strings borrow from that input.
//! The parser does not implement browser tree construction or track SVG/MathML
//! namespaces. It can treat markup as text where a browser creates elements.
//! Do not use it to decide whether HTML is safe to render.

mod document;
mod entities;
mod entities_data;
mod reader;
mod tokenizer;

pub use document::{Document, Element, Error, Limits};
pub use entities::{decode, normalize};
pub use reader::Reader;
pub use tokenizer::{Attribute, Doctype, State, Tag, Token, Tokenizer};
