/// A struct to control how the Lua code is generated. Content can be pushed into the
/// generator and it will automatically add spaces when necessary.
pub struct LuaGenerator {
    column_span: usize,
    current_line_length: usize,
    output: String,
}

fn is_relevant_for_spacing(character: &char) -> bool {
    character.is_ascii_alphabetic() || character.is_digit(10) || *character == '_'
}

impl LuaGenerator {
    /// Creates a generator that will wrap the code on a new line after the amount of
    /// characters given by the `column_span` argument.
    pub fn new(column_span: usize) -> Self {
        Self {
            column_span,
            current_line_length: 0,
            output: String::new(),
        }
    }

    /// Appends a string to the current content of the LuaGenerator. A space may be added
    /// depending of the last character of the current content and the first character pushed.
    pub fn push_str(&mut self, content: &str) {
        if let Some(next_char) = content.chars().next() {
            self.push_space_if_needed(next_char, content.len());

            self.output.push_str(content);
            self.current_line_length += content.len();
        }
    }

    /// Same as the `push_str` function, but for a single character.
    pub fn push_char(&mut self, character: char) {
        self.push_space_if_needed(character, 1);

        self.output.push(character);
        self.current_line_length += 1;
    }

    /// This function pushes a character into the string, without appending a new line
    /// character if the line is about to exceed the column span amount.
    pub fn push_char_force_without_space(&mut self, character: char) {
        self.output.push(character);
        self.current_line_length += 1;
    }

    fn push_space_if_needed(&mut self, next_character: char, pushed_length: usize) {
        if self.current_line_length >= self.column_span {
            self.push_new_line();
        } else {
            let total_length = self.current_line_length + pushed_length;

            if self.needs_space(next_character) {
                if total_length + 1 > self.column_span {
                    self.push_new_line();
                } else {
                    self.output.push(' ');
                    self.current_line_length += 1;
                }
            } else {
                if total_length > self.column_span {
                    self.push_new_line();
                }
            }
        }
    }

    #[inline]
    fn push_new_line(&mut self) {
        self.output.push('\n');
        self.current_line_length = 0;
    }

    fn needs_space(&self, next_character: char) -> bool {
        is_relevant_for_spacing(&next_character)
        && self.output.chars().last().filter(is_relevant_for_spacing).is_some()
    }

    /// Consumes the LuaGenerator and produce a String object.
    pub fn into_string(self) -> String {
        self.output
    }

    /// A utility function to iterate on a vector and call the `for_each` function with each
    /// element of the vector and the `between` function between each element. It is useful
    /// when generating lists separated with a comma.
    pub fn for_each_and_between<T, F, G>(&mut self, vector: &Vec<T>, mut for_each: F, mut between: G)
        where F: FnMut(&mut Self, &T), G: FnMut(&mut Self)
    {
        let last_index = vector.len().checked_sub(1).unwrap_or(0);

        vector.iter().enumerate().for_each(|(index, expression)| {
            for_each(self, expression);

            if index != last_index {
                between(self);
            }
        })
    }
}

impl Default for LuaGenerator {
    fn default() -> Self {
        Self::new(80)
    }
}

/// A trait to convert the abstract syntax tree nodes into Lua code.
pub trait ToLua {
    fn to_lua(&self, generator: &mut LuaGenerator);

    fn to_lua_string(&self) -> String {
        let mut generator = LuaGenerator::default();

        self.to_lua(&mut generator);

        generator.into_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unused_generator_gives_empty_string() {
        let generator = LuaGenerator::default();

        assert_eq!(&generator.into_string(), "");
    }

    #[test]
    fn pushed_single_string_gives_same_string() {
        let mut generator = LuaGenerator::default();
        let content = "hello";

        generator.push_str(content);

        assert_eq!(&generator.into_string(), content);
    }

    #[test]
    fn push_adds_space_between_letters() {
        let mut generator = LuaGenerator::default();
        let content = "hello";

        generator.push_str(content);
        generator.push_str(content);

        assert_eq!(generator.into_string(), format!("{} {}", content, content));
    }

    #[test]
    fn push_adds_space_between_numbers() {
        let mut generator = LuaGenerator::default();
        let content = "12";

        generator.push_str(content);
        generator.push_str(content);

        assert_eq!(generator.into_string(), format!("{} {}", content, content));
    }

    #[test]
    fn push_adds_space_between_underscores() {
        let mut generator = LuaGenerator::default();
        let content = "_";

        generator.push_str(content);
        generator.push_str(content);

        assert_eq!(generator.into_string(), format!("{} {}", content, content));
    }

    #[test]
    fn push_does_not_add_space_between_letters_and_symbol() {
        let mut generator = LuaGenerator::default();
        let content = "hello";

        generator.push_str(content);
        generator.push_str("()");

        assert_eq!(generator.into_string(), format!("{}()", content));
    }
}
