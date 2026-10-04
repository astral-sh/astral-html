//! HTML text-state selection for source-order readers.

use crate::{Attribute, State, Token, Tokenizer};

/// A source-order HTML event reader that selects text modes from start tags.
///
/// HTML text elements such as `script`, `style`, and `textarea` select the
/// corresponding tokenizer state. Scripting is disabled (`noscript` is ordinary
/// markup). Tree construction, SVG/MathML namespace transitions, and fragment
/// insertion modes are not implemented. Use [`Tokenizer`] for explicit context.
///
/// Events borrow unchanged input. The reader retains no tokens and imposes no
/// resource limits. Once exhausted, it keeps returning `None`.
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

    /// Select text states while applying
    /// [`Tokenizer::next_with_attribute_buffer`]'s buffer contract.
    pub(crate) fn next_with_attribute_buffer(
        &mut self,
        attribute_buffer: &mut Vec<Attribute<'a>>,
    ) -> Option<Token<'a>> {
        let token = self.tokenizer.next_with_attribute_buffer(attribute_buffer);
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
        self.next_with_attribute_buffer(&mut Vec::new())
    }
}

impl std::iter::FusedIterator for Reader<'_> {}
