// runtime/nillang/src/parser.rs — Parser for NilLang source files
use crate::ast::*;

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let rest = &self.input[self.pos..];
            if rest.starts_with("//") {
                if let Some(nl) = rest.find('\n') {
                    self.pos += nl + 1;
                } else {
                    self.pos = self.input.len();
                }
            } else if rest.starts_with("/*") {
                if let Some(end) = rest.find("*/") {
                    self.pos += end + 2;
                } else {
                    self.pos = self.input.len();
                }
            } else {
                let c = rest.chars().next().unwrap();
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }
        }
    }

    fn advance_char(&mut self) {
        if let Some(c) = self.input[self.pos..].chars().next() {
            self.pos += c.len_utf8();
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.skip_whitespace();
        self.input[self.pos..].chars().next()
    }

    fn peek_word(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let s = &self.input[self.pos..];
        let mut len = 0;
        for c in s.chars() {
            if c.is_alphanumeric() || c == '_' || c == '@' {
                len += c.len_utf8();
            } else {
                break;
            }
        }
        if len > 0 {
            Some(&s[..len])
        } else {
            None
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        self.skip_whitespace();
        if let Some(c) = self.peek_char() {
            if c == expected {
                self.pos += c.len_utf8();
                return true;
            }
        }
        false
    }

    fn next_word(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let word = self.peek_word()?;
        self.pos += word.len();
        Some(word)
    }

    fn parse_string_literal(&mut self) -> Option<String> {
        self.skip_whitespace();
        let rest = &self.input[self.pos..];
        if !rest.starts_with('"') {
            return None;
        }
        let after_quote = &rest[1..];
        let mut end = 0;
        let mut escaped = false;
        let mut out = String::new();
        for (i, c) in after_quote.char_indices() {
            if escaped {
                out.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                end = i + 1;
                break;
            } else {
                out.push(c);
            }
        }
        if end > 0 {
            self.pos += 1 + end;
            Some(out)
        } else {
            None
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut program = Program {
            imports: Vec::new(),
            structs: Vec::new(),
            apps: Vec::new(),
            components: Vec::new(),
        };

        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }

            let word = match self.peek_word() {
                Some(w) => w,
                None => {
                    self.advance_char();
                    continue;
                }
            };

            match word {
                "import" => {
                    self.next_word();
                    let imp = self.parse_import()?;
                    program.imports.push(imp);
                }
                "struct" => {
                    self.next_word();
                    let st = self.parse_struct()?;
                    program.structs.push(st);
                }
                "app" => {
                    self.next_word();
                    let app = self.parse_app()?;
                    program.apps.push(app);
                }
                "component" => {
                    self.next_word();
                    let comp = self.parse_component()?;
                    program.components.push(comp);
                }
                _ => {
                    self.pos += word.len();
                }
            }
        }

        Ok(program)
    }

    fn parse_import(&mut self) -> Result<ImportStmt, String> {
        let mut symbols = Vec::new();
        if self.consume_char('{') {
            while let Some(w) = self.next_word() {
                if w != "," && w != "}" {
                    symbols.push(w.to_string());
                }
                if self.consume_char('}') {
                    break;
                }
                self.consume_char(',');
            }
        }

        let from_word = self.next_word().unwrap_or("");
        if from_word != "from" {
            return Err("Expected 'from' after import symbols".to_string());
        }

        let source = self.parse_string_literal().unwrap_or_default();
        Ok(ImportStmt { symbols, source })
    }

    fn parse_struct(&mut self) -> Result<StructDef, String> {
        let name = self.next_word().ok_or("Expected struct name")?.to_string();
        if !self.consume_char('{') {
            return Err("Expected '{' after struct name".to_string());
        }

        let mut fields = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }
            let field_name = match self.next_word() {
                Some(w) => w.to_string(),
                None => break,
            };
            self.consume_char(':');
            let field_type = self.next_word().unwrap_or("any").to_string();
            fields.push(StructField { name: field_name, field_type });
            self.consume_char(';');
        }

        Ok(StructDef { name, fields })
    }

    fn parse_app(&mut self) -> Result<AppDef, String> {
        let name = self.next_word().ok_or("Expected app name")?.to_string();
        if !self.consume_char('{') {
            return Err("Expected '{' after app name".to_string());
        }

        let mut state_vars = Vec::new();
        let mut root_ui = None;

        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }

            let word = match self.peek_word() {
                Some(w) => w,
                None => { self.advance_char(); continue; }
            };

            if word == "@State" {
                self.next_word();
                let vname = self.next_word().unwrap_or("").to_string();
                self.consume_char(':');
                let vtype = self.next_word().unwrap_or("string").to_string();
                let mut vval = String::new();
                if self.consume_char('=') {
                    if let Some(s) = self.parse_string_literal() {
                        vval = s;
                    } else {
                        vval = self.next_word().unwrap_or("").to_string();
                    }
                }
                state_vars.push(StateVar { name: vname, var_type: vtype, initial_value: vval });
            } else if word == "build" {
                self.next_word();
                if self.consume_char('(') { self.consume_char(')'); }
                if self.consume_char('{') {
                    if let Ok(ui) = self.parse_ui_element() {
                        root_ui = Some(ui);
                    }
                    while self.pos < self.input.len() {
                        self.skip_whitespace();
                        if self.peek_char() == Some('}') {
                            self.consume_char('}');
                            break;
                        }
                        self.advance_char();
                    }
                }
            } else {
                self.next_word();
            }
        }

        Ok(AppDef { name, state_vars, root_ui })
    }

    fn parse_component(&mut self) -> Result<ComponentDef, String> {
        let name = self.next_word().ok_or("Expected component name")?.to_string();
        if !self.consume_char('{') {
            return Err("Expected '{' after component name".to_string());
        }

        let mut props = Vec::new();
        let mut root_ui = None;

        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }

            let word = match self.peek_word() {
                Some(w) => w,
                None => { self.advance_char(); continue; }
            };

            if word == "prop" {
                self.next_word();
                let pname = self.next_word().unwrap_or("").to_string();
                self.consume_char(':');
                let ptype = self.next_word().unwrap_or("any").to_string();
                props.push(StructField { name: pname, field_type: ptype });
            } else if word == "build" {
                self.next_word();
                if self.consume_char('(') { self.consume_char(')'); }
                if self.consume_char('{') {
                    if let Ok(ui) = self.parse_ui_element() {
                        root_ui = Some(ui);
                    }
                    while self.pos < self.input.len() {
                        self.skip_whitespace();
                        if self.peek_char() == Some('}') {
                            self.consume_char('}');
                            break;
                        }
                        self.advance_char();
                    }
                }
            } else {
                self.next_word();
            }
        }

        Ok(ComponentDef { name, props, root_ui })
    }

    fn parse_ui_element(&mut self) -> Result<UIElement, String> {
        self.skip_whitespace();
        let tag = self.next_word().ok_or("Expected UI element tag")?.to_string();
        let mut text_content = None;

        if self.consume_char('(') {
            if let Some(s) = self.parse_string_literal() {
                text_content = Some(s);
            }
            self.consume_char(')');
        }

        let mut properties = Vec::new();
        let mut children = Vec::new();

        if self.consume_char('{') {
            while self.pos < self.input.len() {
                self.skip_whitespace();
                if self.consume_char('}') {
                    break;
                }

                // Check if it's a child UI element or property
                let word = match self.peek_word() {
                    Some(w) => w,
                    None => { self.advance_char(); continue; }
                };

                // Common declarative tags
                if ["Column", "Row", "Stack", "Text", "Button", "Input", "Progress", "WebView", "LazyList"].contains(&word) {
                    if let Ok(child) = self.parse_ui_element() {
                        children.push(child);
                    }
                } else {
                    let prop_name = self.next_word().unwrap_or("").to_string();
                    if self.consume_char(':') {
                        let val = if let Some(s) = self.parse_string_literal() {
                            s
                        } else {
                            self.next_word().unwrap_or("").to_string()
                        };
                        properties.push(UIProperty { name: prop_name, value: val });
                    }
                }
            }
        }

        Ok(UIElement { tag, text_content, properties, children })
    }
}
