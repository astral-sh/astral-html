//! HTML text-state selection for source-order readers.

use crate::tokenizer::AttributeBudget;
use crate::{Attribute, State, Token, Tokenizer};

/// A source-order HTML event reader that selects text modes from start tags.
///
/// `script`, `style`, `title`, `textarea`, and other HTML text elements are
/// handled without interpreting their contents as nested markup. Scripting is
/// disabled (`noscript` is ordinary markup). This is an HTML-only lexical
/// reader: it does not implement tree construction, SVG/MathML namespace
/// transitions, or fragment insertion modes. Use [`Tokenizer`] to select an
/// explicit tokenizer state when a caller supplies that context.
///
/// Events borrow unchanged strings from the input. The reader does not retain
/// returned tokens or impose resource limits. Once exhausted, it keeps returning
/// `None`.
pub struct Reader<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> Reader<'a> {
    /// Read an already decoded UTF-8 HTML input.
    pub fn new(source: &'a str) -> Self {
        Self {
            tokenizer: Tokenizer::new(source),
        }
    }

    /// Return the consumed byte offset in the original UTF-8 input.
    ///
    /// This includes skipped input and incomplete tags discarded at EOF.
    pub fn position(&self) -> usize {
        self.tokenizer.position()
    }

    /// Read a token while charging parsed attributes to a document's budget.
    ///
    /// Incomplete tags consume budget even when EOF discards them. Exhaustion
    /// returns `None`; the caller must check the budget after the token loop.
    /// The attribute buffer must be empty; returned tags may take ownership of it.
    pub(crate) fn next_with_attribute_budget(
        &mut self,
        budget: &mut AttributeBudget,
        attribute_buffer: &mut Vec<Attribute<'a>>,
    ) -> Option<Token<'a>> {
        let token = self
            .tokenizer
            .next_with_attribute_budget(budget, attribute_buffer);
        if let Some(Token::StartTag(tag)) = &token {
            let state = match tag.name.as_ref() {
                "title" | "textarea" => Some(State::Rcdata),
                "style" | "xmp" | "iframe" | "noembed" | "noframes" => Some(State::Rawtext),
                "script" => Some(State::ScriptData),
                "plaintext" => Some(State::Plaintext),
                _ => None,
            };
            if let Some(state) = state {
                self.tokenizer.set_state(state, Some(&tag.name));
            }
        }
        token
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_with_attribute_budget(&mut AttributeBudget::new(usize::MAX), &mut Vec::new())
    }
}

impl std::iter::FusedIterator for Reader<'_> {}
