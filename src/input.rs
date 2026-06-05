//! Single-line text editor used by sidebar and dialog fields.

/// Editable UTF-8 string with a logical (character) cursor position.
#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    /// Create an input with the cursor at the end of `value`.
    ///
    /// # Arguments
    ///
    /// * `value` - Initial text content.
    ///
    /// # Returns
    ///
    /// A new `TextInput` with `cursor` at the end of the string.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    /// Borrow the current text.
    ///
    /// # Returns
    ///
    /// A string slice of the buffer contents.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Number of Unicode scalar values in the buffer.
    ///
    /// # Returns
    ///
    /// Character count (not byte length).
    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    /// Insert `c` before the cursor and advance the cursor by one character.
    ///
    /// # Arguments
    ///
    /// * `c` - Character to insert at the cursor position.
    pub fn insert(&mut self, c: char) {
        let idx = char_to_byte_index(&self.value, self.cursor);
        self.value.insert(idx, c);
        self.cursor += 1;
    }

    /// Delete the character immediately before the cursor.
    ///
    /// No-op when the cursor is at the start.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let start = char_to_byte_index(&self.value, self.cursor);
        let end = char_to_byte_index(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    /// Delete the character at the cursor without moving it.
    ///
    /// No-op when the cursor is at the end.
    pub fn delete(&mut self) {
        if self.cursor >= self.char_len() {
            return;
        }
        let start = char_to_byte_index(&self.value, self.cursor);
        let end = char_to_byte_index(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    /// Move the cursor one character left, stopping at the start.
    pub fn cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Move the cursor one character right, stopping at the end.
    pub fn cursor_right(&mut self) {
        if self.cursor < self.char_len() {
            self.cursor += 1;
        }
    }

    /// Move the cursor to the beginning of the string.
    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    /// Move the cursor to the end of the string.
    pub fn cursor_end(&mut self) {
        self.cursor = self.char_len();
    }
}

/// Map a character index to a byte offset in `s`.
///
/// # Arguments
///
/// * `s` - UTF-8 string to index into.
/// * `char_idx` - Logical character position.
///
/// # Returns
///
/// Byte offset suitable for `str` slicing, or `s.len()` if `char_idx` is past the end.
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_places_cursor_at_end() {
        let input = TextInput::new("abc");
        assert_eq!(input.cursor, 3);
        assert_eq!(input.as_str(), "abc");
    }

    #[test]
    fn insert_at_cursor() {
        let mut input = TextInput::new("ac");
        input.cursor = 1;
        input.insert('b');
        assert_eq!(input.as_str(), "abc");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut input = TextInput::new("a");
        input.cursor = 0;
        input.backspace();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn backspace_removes_previous_char() {
        let mut input = TextInput::new("ab");
        input.cursor = 2;
        input.backspace();
        assert_eq!(input.as_str(), "a");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn delete_at_end_is_noop() {
        let mut input = TextInput::new("ab");
        input.cursor = 2;
        input.delete();
        assert_eq!(input.as_str(), "ab");
    }

    #[test]
    fn delete_removes_char_at_cursor() {
        let mut input = TextInput::new("abc");
        input.cursor = 1;
        input.delete();
        assert_eq!(input.as_str(), "ac");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn cursor_left_stops_at_start() {
        let mut input = TextInput::new("ab");
        input.cursor_left();
        input.cursor_left();
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn cursor_right_stops_at_end() {
        let mut input = TextInput::new("ab");
        input.cursor_end();
        input.cursor_right();
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn cursor_home_and_end() {
        let mut input = TextInput::new("hello");
        input.cursor_home();
        assert_eq!(input.cursor, 0);
        input.cursor_end();
        assert_eq!(input.cursor, 5);
    }

    #[test]
    fn unicode_cursor_positions() {
        let mut input = TextInput::new("aéb");
        assert_eq!(input.char_len(), 3);
        input.cursor = 1;
        input.insert('ñ');
        assert_eq!(input.as_str(), "añéb");
        assert_eq!(input.char_len(), 4);
        input.cursor = 2;
        input.backspace();
        assert_eq!(input.as_str(), "aéb");
    }
}
