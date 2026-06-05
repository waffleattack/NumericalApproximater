#[derive(Debug, Clone)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    pub fn insert(&mut self, c: char) {
        let idx = char_to_byte_index(&self.value, self.cursor);
        self.value.insert(idx, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let start = char_to_byte_index(&self.value, self.cursor);
        let end = char_to_byte_index(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.char_len() {
            return;
        }
        let start = char_to_byte_index(&self.value, self.cursor);
        let end = char_to_byte_index(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    pub fn cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn cursor_right(&mut self) {
        if self.cursor < self.char_len() {
            self.cursor += 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor = self.char_len();
    }
}

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
