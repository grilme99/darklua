use crate::generator::{utils, LuaGenerator};
use crate::nodes::{self, Identifier};

/// This implementation of [LuaGenerator](trait.LuaGenerator.html) attempts to produce Lua code as
/// small as possible. It is not meant to be read by humans.
#[derive(Debug, Clone)]
pub struct DenseLuaGenerator {
    column_span: usize,
    current_line_length: usize,
    output: String,
    last_push_length: usize,
}

impl DenseLuaGenerator {
    /// Creates a generator that will wrap the code on a new line after the amount of
    /// characters given by the `column_span` argument.
    pub fn new(column_span: usize) -> Self {
        Self {
            column_span,
            current_line_length: 0,
            output: String::new(),
            last_push_length: 0,
        }
    }

    /// Appends a string to the current content of the DenseLuaGenerator. A space may be added
    /// depending of the last character of the current content and the first character pushed.
    fn push_str(&mut self, content: &str) {
        if let Some(next_char) = content.chars().next() {
            self.push_space_if_needed(next_char, content.len());
            self.raw_push_str(content);
        }
    }

    /// Same as the `push_str` function, but for a single character.
    fn push_char(&mut self, character: char) {
        self.push_space_if_needed(character, 1);

        self.output.push(character);
        self.current_line_length += 1;
        self.last_push_length = 1;
    }

    /// This function pushes a character into the string, without appending a new line
    /// or a space between the last pushed content.
    fn merge_char(&mut self, character: char) {
        if self.fits_on_current_line(1) {
            self.raw_push_char(character);
        } else {
            let last_push_content = self.get_last_push_str().to_owned();
            (0..self.last_push_length).for_each(|_| {
                self.output.pop();
            });

            let mut last_char = self.output.pop();

            while let Some(' ') = last_char {
                last_char = self.output.pop();
            }

            if let Some(last_char) = last_char {
                self.output.push(last_char);
            }

            self.output.push('\n');
            self.output.push_str(&last_push_content);
            self.output.push(character);
            self.last_push_length += 1;
            self.current_line_length = self.last_push_length;
        }
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
            } else if total_length > self.column_span {
                self.push_new_line();
            }
        }
    }

    #[inline]
    fn push_new_line(&mut self) {
        self.output.push('\n');
        self.current_line_length = 0;
    }

    #[inline]
    fn push_space(&mut self) {
        self.output.push(' ');
        self.current_line_length += 1;
    }

    #[inline]
    fn fits_on_current_line(&self, length: usize) -> bool {
        self.current_line_length + length <= self.column_span
    }

    #[inline]
    fn needs_space(&self, next_character: char) -> bool {
        utils::is_relevant_for_spacing(&next_character)
            && self
                .output
                .chars()
                .last()
                .filter(utils::is_relevant_for_spacing)
                .is_some()
    }

    /// Consumes the LuaGenerator and produce a String object.
    pub fn into_string(self) -> String {
        self.output
    }

    #[inline]
    fn raw_push_str(&mut self, content: &str) {
        self.output.push_str(content);
        self.last_push_length = content.len();
        self.current_line_length += self.last_push_length;
    }

    #[inline]
    fn raw_push_char(&mut self, character: char) {
        self.output.push(character);
        self.last_push_length = 1;
        self.current_line_length += 1;
    }

    /// This function only insert a space or a new line if the given predicate returns true. In
    /// the other case, the string is added to the current generator content.
    fn push_str_and_break_if<F>(&mut self, content: &str, predicate: F)
    where
        F: Fn(&str) -> bool,
    {
        if predicate(self.get_last_push_str()) {
            if self.fits_on_current_line(1 + content.len()) {
                self.push_space();
            } else {
                self.push_new_line();
            }
        } else if !self.fits_on_current_line(content.len()) {
            self.push_new_line();
        }
        self.raw_push_str(content);
    }

    fn get_last_push_str(&self) -> &str {
        self.output
            .get((self.output.len() - self.last_push_length)..)
            .unwrap_or("")
    }

    fn write_function_parameters(&mut self, parameters: &[Identifier], is_variadic: bool) {
        let last_index = parameters.len().saturating_sub(1);

        parameters.iter().enumerate().for_each(|(index, variable)| {
            self.push_str(variable.get_name());

            if index != last_index {
                self.push_char(',');
            }
        });

        if is_variadic {
            if !parameters.is_empty() {
                self.push_char(',');
            };
            self.push_str("...");
        };
    }
}

impl Default for DenseLuaGenerator {
    fn default() -> Self {
        Self::new(80)
    }
}

impl LuaGenerator for DenseLuaGenerator {
    /// Consumes the LuaGenerator and produce a String object.
    fn into_string(self) -> String {
        self.output
    }

    fn write_block(&mut self, block: &nodes::Block) {
        let mut statements = block.iter_statements().peekable();

        while let Some(statement) = statements.next() {
            self.write_statement(statement);

            if let Some(next_statement) = statements.peek() {
                if utils::starts_with_parenthese(next_statement)
                    && utils::ends_with_prefix(statement)
                {
                    self.push_char(';');
                }
            }
        }

        if let Some(last_statement) = block.get_last_statement() {
            self.write_last_statement(last_statement);
        }
    }

    fn write_assign_statement(&mut self, assign: &nodes::AssignStatement) {
        let variables = assign.get_variables();
        let last_variable_index = variables.len() - 1;

        variables.iter().enumerate().for_each(|(index, variable)| {
            self.write_variable(variable);

            if index != last_variable_index {
                self.push_char(',');
            }
        });

        self.push_char('=');

        let last_value_index = assign.values_len() - 1;

        assign.iter_values().enumerate().for_each(|(index, value)| {
            self.write_expression(value);

            if index != last_value_index {
                self.push_char(',');
            }
        });
    }

    fn write_do_statement(&mut self, do_statement: &nodes::DoStatement) {
        self.push_str("do");
        self.write_block(do_statement.get_block());
        self.push_str("end");
    }

    fn write_generic_for(&mut self, generic_for: &nodes::GenericForStatement) {
        self.push_str("for");

        let identifiers = generic_for.get_identifiers();
        let last_identifier_index = identifiers.len().saturating_sub(1);
        identifiers
            .iter()
            .enumerate()
            .for_each(|(index, identifier)| {
                self.push_str(identifier.get_name());

                if index != last_identifier_index {
                    self.push_char(',');
                }
            });
        self.push_str("in");

        let expressions = generic_for.get_expressions();
        let last_expression_index = expressions.len().saturating_sub(1);
        expressions
            .iter()
            .enumerate()
            .for_each(|(index, expression)| {
                self.write_expression(expression);

                if index != last_expression_index {
                    self.push_char(',');
                }
            });

        self.push_str("do");
        self.write_block(generic_for.get_block());
        self.push_str("end");
    }

    fn write_if_statement(&mut self, if_statement: &nodes::IfStatement) {
        let branches = if_statement.get_branches();

        branches.iter().enumerate().for_each(|(index, branch)| {
            if index == 0 {
                self.push_str("if");
            } else {
                self.push_str("elseif");
            }

            self.write_expression(branch.get_condition());
            self.push_str("then");
            self.write_block(branch.get_block());
        });

        if let Some(else_block) = if_statement.get_else_block() {
            self.push_str("else");
            self.write_block(else_block)
        }

        self.push_str("end");
    }

    fn write_function_statement(&mut self, function: &nodes::FunctionStatement) {
        self.push_str("function");
        let name = function.get_name();

        self.push_str(name.get_name().get_name());
        name.get_field_names().iter().for_each(|field| {
            self.push_char('.');
            self.push_str(field.get_name());
        });

        if let Some(method) = name.get_method() {
            self.push_char(':');
            self.push_str(method.get_name());
        }

        self.push_char('(');
        self.write_function_parameters(function.get_parameters(), function.is_variadic());
        self.push_char(')');

        let block = function.get_block();

        if !block.is_empty() {
            self.write_block(block);
        }
        self.push_str("end");
    }

    fn write_last_statement(&mut self, statement: &nodes::LastStatement) {
        use nodes::LastStatement::*;

        match statement {
            Break(_) => self.push_str("break"),
            Continue(_) => self.push_str("continue"),
            Return(expressions) => {
                self.push_str("return");
                let last_index = expressions.len().saturating_sub(1);

                expressions
                    .iter_expressions()
                    .enumerate()
                    .for_each(|(index, expression)| {
                        self.write_expression(expression);

                        if index != last_index {
                            self.push_char(',');
                        }
                    });
            }
        }
    }

    fn write_local_assign(&mut self, assign: &nodes::LocalAssignStatement) {
        self.push_str("local");

        let variables = assign.get_variables();
        let last_variable_index = variables.len().saturating_sub(1);

        variables.iter().enumerate().for_each(|(index, variable)| {
            self.push_str(variable.get_name());

            if index != last_variable_index {
                self.push_char(',');
            }
        });

        if assign.has_values() {
            self.push_char('=');

            let last_value_index = assign.values_len() - 1;

            assign.iter_values().enumerate().for_each(|(index, value)| {
                self.write_expression(value);

                if index != last_value_index {
                    self.push_char(',');
                }
            });
        };
    }

    fn write_compound_assign(&mut self, assign: &nodes::CompoundAssignStatement) {
        self.write_variable(assign.get_variable());

        self.push_str(assign.get_operator().to_str());

        self.write_expression(assign.get_value());
    }

    fn write_local_function(&mut self, function: &nodes::LocalFunctionStatement) {
        self.push_str("local function");
        self.push_str(function.get_name());
        self.push_char('(');

        let parameters = function.get_parameters();
        self.write_function_parameters(parameters, function.is_variadic());
        self.push_char(')');

        let block = function.get_block();

        if !block.is_empty() {
            self.write_block(block);
        }
        self.push_str("end");
    }

    fn write_numeric_for(&mut self, numeric_for: &nodes::NumericForStatement) {
        self.push_str("for");

        self.push_str(numeric_for.get_identifier().get_name());
        self.push_char('=');
        self.write_expression(numeric_for.get_start());
        self.push_char(',');
        self.write_expression(numeric_for.get_end());

        if let Some(step) = numeric_for.get_step() {
            self.push_char(',');
            self.write_expression(step);
        }

        let block = numeric_for.get_block();

        if block.is_empty() {
            self.push_str("do end");
        } else {
            self.push_str("do");
            self.write_block(block);
            self.push_str("end");
        }
    }

    fn write_repeat_statement(&mut self, repeat: &nodes::RepeatStatement) {
        self.push_str("repeat");

        let block = repeat.get_block();

        if !block.is_empty() {
            self.write_block(block);
        }

        self.push_str("until");
        self.write_expression(repeat.get_condition());
    }

    fn write_while_statement(&mut self, while_statement: &nodes::WhileStatement) {
        self.push_str("while");
        self.write_expression(while_statement.get_condition());

        let block = while_statement.get_block();

        if block.is_empty() {
            self.push_str("do end");
        } else {
            self.push_str("do");
            self.write_block(block);
            self.push_str("end");
        }
    }

    fn write_expression(&mut self, expression: &nodes::Expression) {
        use nodes::Expression::*;
        match expression {
            Binary(binary) => self.write_binary_expression(binary),
            Call(call) => self.write_function_call(call),
            False(_) => self.push_str("false"),
            Field(field) => self.write_field(field),
            Function(function) => self.write_function(function),
            Identifier(identifier) => self.write_identifier(identifier),
            If(if_expression) => self.write_if_expression(if_expression),
            Index(index) => self.write_index(index),
            Nil(_) => self.push_str("nil"),
            Number(number) => self.write_number(number),
            Parenthese(parenthese) => self.write_parenthese(parenthese),
            String(string) => self.write_string(string),
            Table(table) => self.write_table(table),
            True(_) => self.push_str("true"),
            Unary(unary) => self.write_unary_expression(unary),
            VariableArguments(_) => {
                self.push_str_and_break_if("...", utils::break_variable_arguments);
            }
        }
    }

    fn write_binary_expression(&mut self, binary: &nodes::BinaryExpression) {
        use nodes::BinaryOperator;

        let operator = binary.operator();
        let left = binary.left();
        let right = binary.right();

        if operator.left_needs_parentheses(left) {
            self.push_char('(');
            self.write_expression(left);
            self.push_char(')');
        } else {
            self.write_expression(left);
        }

        match operator {
            BinaryOperator::Concat => self.push_str_and_break_if("..", utils::break_concat),
            _ => self.push_str(operator.to_str()),
        }

        if operator.right_needs_parentheses(right) {
            self.push_char('(');
            self.write_expression(right);
            self.push_char(')');
        } else {
            self.write_expression(right);
        }
    }

    fn write_unary_expression(&mut self, unary: &nodes::UnaryExpression) {
        use nodes::{Expression, UnaryOperator::*};

        match unary.operator() {
            Length => self.push_char('#'),
            Minus => self.push_str_and_break_if("-", utils::break_minus),
            Not => self.push_str("not"),
        }

        let expression = unary.get_expression();

        match expression {
            Expression::Binary(binary) if !binary.operator().precedes_unary_expression() => {
                self.push_char('(');
                self.write_expression(expression);
                self.push_char(')');
            }
            _ => self.write_expression(expression),
        }
    }

    fn write_function(&mut self, function: &nodes::FunctionExpression) {
        self.push_str("function");
        self.push_char('(');

        let parameters = function.get_parameters();
        self.write_function_parameters(parameters, function.is_variadic());
        self.push_char(')');

        let block = function.get_block();

        if !block.is_empty() {
            self.write_block(block);
        }
        self.push_str("end");
    }

    fn write_function_call(&mut self, call: &nodes::FunctionCall) {
        self.write_prefix(call.get_prefix());

        if let Some(method) = &call.get_method() {
            self.push_char(':');
            self.push_str(method.get_name());
        }

        self.write_arguments(call.get_arguments());
    }

    fn write_field(&mut self, field: &nodes::FieldExpression) {
        self.write_prefix(field.get_prefix());

        self.push_char('.');
        self.push_str(field.get_field().get_name());
    }

    fn write_index(&mut self, index: &nodes::IndexExpression) {
        self.write_prefix(index.get_prefix());

        self.push_char('[');
        self.write_expression(index.get_index());
        self.push_char(']');
    }

    fn write_if_expression(&mut self, if_expression: &nodes::IfExpression) {
        self.push_str("if");
        self.write_expression(if_expression.get_condition());
        self.push_str("then");
        self.write_expression(if_expression.get_result());

        for branch in if_expression.iter_branches() {
            self.push_str("elseif");
            self.write_expression(branch.get_condition());
            self.push_str("then");
            self.write_expression(branch.get_result());
        }

        self.push_str("else");
        self.write_expression(if_expression.get_else_result());
    }

    fn write_table(&mut self, table: &nodes::TableExpression) {
        self.push_char('{');

        let entries = table.get_entries();
        let last_index = entries.len().saturating_sub(1);

        entries.iter().enumerate().for_each(|(index, entry)| {
            self.write_table_entry(entry);

            if index != last_index {
                self.push_char(',');
            }
        });

        self.push_char('}');
    }

    fn write_table_entry(&mut self, entry: &nodes::TableEntry) {
        match entry {
            nodes::TableEntry::Field(entry) => {
                self.push_str(entry.get_field().get_name());
                self.push_char('=');
                self.write_expression(entry.get_value());
            }
            nodes::TableEntry::Index(entry) => {
                self.push_char('[');
                self.write_expression(entry.get_key());
                self.push_char(']');
                self.push_char('=');
                self.write_expression(entry.get_value());
            }
            nodes::TableEntry::Value(expression) => self.write_expression(expression),
        }
    }

    fn write_number(&mut self, number: &nodes::NumberExpression) {
        use nodes::NumberExpression::*;

        match number {
            Decimal(number) => {
                let float = number.get_raw_float();
                if float.is_nan() {
                    self.push_char('(');
                    self.push_char('0');
                    self.push_char('/');
                    self.push_char('0');
                    self.push_char(')');
                } else if float.is_infinite() {
                    self.push_char('(');
                    if float.is_sign_negative() {
                        self.push_char('-');
                    }
                    self.push_char('1');
                    self.push_char('/');
                    self.push_char('0');
                    self.push_char(')');
                } else {
                    let mut result = format!("{}", float);

                    if let Some(exponent) = number.get_exponent() {
                        let exponent_char = number
                            .is_uppercase()
                            .map(|is_uppercase| if is_uppercase { 'E' } else { 'e' })
                            .unwrap_or('e');

                        result.push(exponent_char);
                        result.push_str(&format!("{}", exponent));
                    };

                    self.push_str(&result);
                }
            }
            Hex(number) => {
                let mut result = format!(
                    "0{}{:x}",
                    if number.is_x_uppercase() { 'X' } else { 'x' },
                    number.get_raw_integer()
                );

                if let Some(exponent) = number.get_exponent() {
                    let exponent_char = number
                        .is_exponent_uppercase()
                        .map(|is_uppercase| if is_uppercase { 'P' } else { 'p' })
                        .unwrap_or('p');

                    result.push(exponent_char);
                    result.push_str(&format!("{}", exponent));
                };

                self.push_str(&result);
            }
            Binary(number) => {
                self.push_str(&format!(
                    "0{}{:b}",
                    if number.is_b_uppercase() { 'B' } else { 'b' },
                    number.get_raw_value()
                ));
            }
        }
    }

    fn write_tuple_arguments(&mut self, arguments: &nodes::TupleArguments) {
        self.merge_char('(');

        let last_index = arguments.len().saturating_sub(1);
        arguments
            .iter_values()
            .enumerate()
            .for_each(|(index, expression)| {
                self.write_expression(expression);

                if index != last_index {
                    self.push_char(',');
                }
            });

        self.push_char(')');
    }

    fn write_string(&mut self, string: &nodes::StringExpression) {
        let result = utils::write_string(string);
        if result.starts_with('[') {
            self.push_str_and_break_if(&result, utils::break_long_string);
        } else {
            self.push_str(&result);
        }
    }

    fn write_identifier(&mut self, identifier: &nodes::Identifier) {
        self.push_str(identifier.get_name());
    }

    fn write_parenthese(&mut self, parenthese: &nodes::ParentheseExpression) {
        self.push_char('(');
        self.write_expression(parenthese.inner_expression());
        self.push_char(')');
    }
}
