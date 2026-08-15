//! Incremental EWTS composition independent of Windows keyboard APIs.

use ewts::EwtsConverter;

/// An edit to apply to the foreground application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// Number of previously emitted Unicode scalar values to erase.
    pub backspaces: usize,
    /// Replacement Unicode text to emit.
    pub text: String,
}

/// Maintains one live EWTS composition span.
///
/// The span is reconverted after every keystroke. This makes ambiguous stacks
/// such as `rgy` display correctly as they are extended.
pub struct Composer {
    converter: EwtsConverter,
    source: String,
    rendered: String,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            converter: EwtsConverter::create(),
            source: String::new(),
            rendered: String::new(),
        }
    }
}

impl Composer {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn rendered(&self) -> &str {
        &self.rendered
    }

    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Add one ASCII EWTS character and return the foreground edit.
    pub fn push(&mut self, ch: char) -> Replacement {
        debug_assert!(ch.is_ascii());
        let old_len = self.rendered.chars().count();
        self.source.push(ch);
        self.rendered = self.converter.ewts_to_unicode(&self.source);
        let edit = Replacement {
            backspaces: old_len,
            text: self.rendered.clone(),
        };

        if is_commit_character(ch) {
            self.source.clear();
            self.rendered.clear();
        }

        edit
    }

    /// Remove one source character, if a composition is active.
    pub fn backspace(&mut self) -> Option<Replacement> {
        self.source.pop()?;
        let old_len = self.rendered.chars().count();
        self.rendered = self.converter.ewts_to_unicode(&self.source);
        Some(Replacement {
            backspaces: old_len,
            text: self.rendered.clone(),
        })
    }

    /// Finish the current span without altering its rendered text.
    pub fn commit(&mut self) {
        self.source.clear();
        self.rendered.clear();
    }
}

/// Characters whose EWTS meaning finishes a composition span.
fn is_commit_character(ch: char) -> bool {
    matches!(ch, ' ' | '_' | '/' | ';' | '|' | '!' | ':' | '\n' | '\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_representative_ewts() {
        let converter = EwtsConverter::create();
        assert_eq!(converter.ewts_to_unicode("sangs rgyas"), "སངས་རྒྱས");
        assert_eq!(converter.ewts_to_unicode("oM"), "ཨོཾ");
        assert_eq!(converter.ewts_to_unicode("bka' brgyud"), "བཀའ་བརྒྱུད");
        assert_eq!(converter.ewts_to_unicode("grA"), "གྲཱ");
    }

    #[test]
    fn incremental_replacement_tracks_rendered_length() {
        let mut composer = Composer::default();
        let first = composer.push('r');
        assert_eq!(
            first,
            Replacement {
                backspaces: 0,
                text: "ར".into()
            }
        );

        let second = composer.push('g');
        assert_eq!(second.backspaces, 1);

        for ch in ['y', 'a', 's'] {
            composer.push(ch);
        }
        assert_eq!(composer.rendered(), "རྒྱས");
    }

    #[test]
    fn space_emits_tsheg_and_commits() {
        let mut composer = Composer::default();
        for ch in "sangs".chars() {
            composer.push(ch);
        }
        let edit = composer.push(' ');
        assert_eq!(edit.text, "སངས་");
        assert!(composer.is_empty());
    }

    #[test]
    fn underscore_emits_word_space_and_commits() {
        let mut composer = Composer::default();
        for ch in "sangs".chars() {
            composer.push(ch);
        }
        let edit = composer.push('_');
        assert_eq!(edit.text, "སངས ");
        assert!(composer.is_empty());
    }

    #[test]
    fn backspace_reconverts_source() {
        let mut composer = Composer::default();
        for ch in "rgyas".chars() {
            composer.push(ch);
        }
        let edit = composer.backspace().unwrap();
        assert_eq!(edit.text, "རྒྱ");
        assert_eq!(composer.source(), "rgya");
    }
}
