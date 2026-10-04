//! HTML text-state selection for source-order readers.

use crate::{Error, State, Token, Tokenizer};

/// A source-order HTML event reader that selects text modes from start tags.
///
/// `script`, `style`, `title`, `textarea`, and other HTML text elements are
/// handled without interpreting their contents as nested markup. Scripting is
/// disabled (`noscript` is ordinary markup). This is an HTML-only lexical
/// reader: it does not implement tree construction, SVG/MathML namespace
/// transitions, or fragment insertion modes. Use [`Tokenizer`] to select an
/// explicit tokenizer state when a caller supplies that context.
///
/// The reader retains only the current token and borrows unchanged strings
/// from its input. The event reader does not impose resource limits.
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
    pub fn position(&self) -> usize {
        self.tokenizer.position()
    }

    /// Read a token while charging parsed attributes to a document's budget.
    pub(crate) fn next_with_attribute_budget(
        &mut self,
        remaining_attributes: &mut usize,
    ) -> Option<Result<Token<'a>, Error>> {
        let token = self
            .tokenizer
            .next_with_attribute_budget(remaining_attributes)?;
        if let Ok(Token::StartTag(tag)) = &token {
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
        Some(token)
    }
}

impl<'a> Iterator for Reader<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut remaining_attributes = usize::MAX;
        self.next_with_attribute_budget(&mut remaining_attributes)
            .map(|token| token.expect("an unbounded reader cannot exhaust its attribute budget"))
    }
}

impl std::iter::FusedIterator for Reader<'_> {}
