use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub struct Schema {
    pub ast: Ast,
    pub params: ParamStack,
}

impl TryFrom<&[u8]> for Schema {
    type Error = SchemaParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let parser = SchemaParser::new(bytes);
        parser.parse()
    }
}

impl FromStr for Schema {
    type Err = SchemaParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self>::try_from(s.as_bytes())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ast {
    pub kind: AstKind,
    pub name: String,
}

impl Ast {
    pub(crate) fn size(&self) -> Size {
        match self.kind {
            AstKind::Int8 => Size::Known(std::mem::size_of::<i8>()),
            AstKind::Int16 => Size::Known(std::mem::size_of::<i16>()),
            AstKind::Int32 => Size::Known(std::mem::size_of::<i32>()),
            AstKind::UInt8 => Size::Known(std::mem::size_of::<u8>()),
            AstKind::UInt16 => Size::Known(std::mem::size_of::<u16>()),
            AstKind::UInt32 => Size::Known(std::mem::size_of::<u32>()),
            AstKind::Float32 => Size::Known(std::mem::size_of::<f32>()),
            AstKind::Float64 => Size::Known(std::mem::size_of::<f64>()),
            AstKind::Str => Size::Unknown,
            AstKind::NStr(size) => Size::Known(size),
            AstKind::Struct { .. } => Size::Undefined,
            AstKind::Array { .. } => Size::Undefined,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AstKind {
    Int8,
    Int16,
    Int32,
    UInt8,
    UInt16,
    UInt32,
    Float32,
    Float64,
    Str,
    NStr(usize),
    Struct(Vec<Ast>),
    Array(Len, Box<Ast>), // use Box to avoid E0072
}

#[derive(Debug, PartialEq, Eq)]
pub enum Len {
    Fixed(usize),
    Variable(String),
}

pub(crate) enum Size {
    Known(usize),
    Unknown,
    Undefined,
}

struct SchemaParser<'b> {
    lexer: std::iter::Peekable<SchemaLexer<'b>>,
    params: ParamStack,
}

impl<'b> SchemaParser<'b> {
    fn new(input: &'b [u8]) -> Self {
        Self {
            lexer: SchemaLexer::new(input).peekable(),
            params: ParamStack::new(),
        }
    }

    fn parse(mut self) -> Result<Schema, SchemaParseError> {
        let kind = self.parse_field_list()?;
        if self.lexer.next().is_some() {
            // should be Token::RBracket
            return Err(SchemaParseError::UnknownError(
                "reading field list finished but some tokens are unexpectedly left".to_owned(),
            ));
        }

        let schema = Schema {
            ast: Ast {
                name: "".to_owned(),
                kind,
            },
            params: self.params,
        };
        Ok(schema)
    }

    fn parse_field_list(&mut self) -> Result<AstKind, SchemaParseError> {
        let mut members = Vec::new();

        while let Some(token) = self.lexer.next() {
            let name = if let Token::Ident(s) = token? {
                s
            } else {
                return Err(SchemaParseError::UnexpectedToken);
            };

            self.consume_symbol(Token::Colon)?;

            let kind = self.parse_type()?;
            let member = Ast { kind, name };
            members.push(member);

            if matches!(self.lexer.peek(), None | Some(Ok(Token::RBracket))) {
                break;
            }

            if !matches!(self.lexer.next(), Some(Ok(Token::Comma))) {
                return Err(SchemaParseError::UnexpectedToken);
            }
        }

        if members.len() == 0 {
            return Err(SchemaParseError::UnexpectedEof);
        }

        let kind = AstKind::Struct(members);
        Ok(kind)
    }

    fn parse_type(&mut self) -> Result<AstKind, SchemaParseError> {
        let token = self.lexer.next();

        match token.unwrap_or(Err(SchemaParseError::UnexpectedEof))? {
            Token::Ident(s) => self.parse_builtin_type(s),
            Token::LBracket => {
                let kind = self.parse_field_list()?;
                // consumes next Token::RBracket or reaches EOF
                if self.lexer.next().is_none() {
                    return Err(SchemaParseError::UnexpectedEof);
                }
                Ok(kind)
            }
            Token::LAngleBracket => self.parse_nstr_type(),
            Token::LBrace => self.parse_array(),
            _ => Err(SchemaParseError::UnexpectedToken),
        }
    }

    fn parse_builtin_type(&mut self, ident: String) -> Result<AstKind, SchemaParseError> {
        let kind = match ident.as_str() {
            "INT8" => AstKind::Int8,
            "INT16" => AstKind::Int16,
            "INT32" => AstKind::Int32,
            "UINT8" => AstKind::UInt8,
            "UINT16" => AstKind::UInt16,
            "UINT32" => AstKind::UInt32,
            "FLOAT32" => AstKind::Float32,
            "FLOAT64" => AstKind::Float64,
            "STR" => AstKind::Str,
            _ => {
                return Err(SchemaParseError::UnknownError(format!(
                    "unknown builtin type {ident}"
                )))
            }
        };
        Ok(kind)
    }

    fn parse_nstr_type(&mut self) -> Result<AstKind, SchemaParseError> {
        // LAngleBracket has already been read
        let len = self.consume_number()?;
        self.consume_symbol(Token::RAngleBracket)?;

        let nstr_ident = self
            .lexer
            .next()
            .unwrap_or(Err(SchemaParseError::UnexpectedEof))?;
        if let Token::Ident(s) = nstr_ident {
            if s.as_str() != "NSTR" {
                return Err(SchemaParseError::UnexpectedToken);
            }
        } else {
            return Err(SchemaParseError::UnexpectedToken);
        }

        let kind = AstKind::NStr(len);
        Ok(kind)
    }

    fn parse_array(&mut self) -> Result<AstKind, SchemaParseError> {
        // LBrace has already been read

        let len = match self
            .lexer
            .next()
            .unwrap_or(Err(SchemaParseError::UnexpectedEof))?
        {
            Token::Number(n) => Len::Fixed(n),
            Token::Ident(s) => {
                self.params.add_entry(&s);
                Len::Variable(s)
            }
            _ => return Err(SchemaParseError::UnexpectedToken),
        };

        self.consume_symbol(Token::RBrace)?;
        self.consume_symbol(Token::LBracket)?;
        let struct_kind = self.parse_field_list()?;
        // consumes next Token::RBracket or reaches EOF
        if self.lexer.next().is_none() {
            return Err(SchemaParseError::UnexpectedEof);
        }

        let struct_node = Ast {
            kind: struct_kind,
            name: "[]".to_owned(),
        };
        Ok(AstKind::Array(len, Box::new(struct_node)))
    }

    fn consume_number(&mut self) -> Result<usize, SchemaParseError> {
        match self
            .lexer
            .next()
            .unwrap_or(Err(SchemaParseError::UnexpectedEof))?
        {
            Token::Number(n) => Ok(n),
            _ => Err(SchemaParseError::UnexpectedToken),
        }
    }

    fn consume_symbol(&mut self, symbol: Token) -> Result<(), SchemaParseError> {
        let token = self.lexer.next();
        if token.is_none() {
            return Err(SchemaParseError::UnexpectedEof);
        } else if token != Some(Ok(symbol)) {
            return Err(SchemaParseError::UnexpectedToken);
        }
        Ok(())
    }
}

struct SchemaLexer<'b> {
    input: &'b [u8],
    pos: usize,
}

impl<'b> SchemaLexer<'b> {
    fn new(input: &'b [u8]) -> Self {
        SchemaLexer { input, pos: 0 }
    }

    fn lex_ident(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len()
            && matches!(self.input[self.pos], b'0'..=b'9' | b'A'..=b'Z'| b'a'..=b'z'| b'_')
        {
            self.pos += 1;
        }
        let token = Token::Ident(String::from_utf8_lossy(&self.input[start..self.pos]).to_string());
        token
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && matches!(self.input[self.pos], b'0'..=b'9') {
            self.pos += 1;
        }
        let token =
            Token::Number((String::from_utf8_lossy(&self.input[start..self.pos]).parse()).unwrap());
        token
    }
}

impl<'b> Iterator for SchemaLexer<'b> {
    type Item = Result<Token, SchemaParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! lex {
            ($tok:expr) => {{
                self.pos += 1;
                Ok($tok)
            }};
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let token = match self.input[self.pos] {
            b'A'..=b'Z' | b'a'..=b'z' => Ok(self.lex_ident()),
            b'1'..=b'9' => Ok(self.lex_number()),
            b':' => lex!(Token::Colon),
            b',' => lex!(Token::Comma),
            b'[' => lex!(Token::LBracket),
            b']' => lex!(Token::RBracket),
            b'<' => lex!(Token::LAngleBracket),
            b'>' => lex!(Token::RAngleBracket),
            b'{' => lex!(Token::LBrace),
            b'}' => lex!(Token::RBrace),
            _ => Err(SchemaParseError::UnknownToken),
        };
        Some(token)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.input.len()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    Number(usize),
    Colon,
    Comma,
    LBracket,
    RBracket,
    LAngleBracket,
    RAngleBracket,
    LBrace,
    RBrace,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParamStack {
    stacks: HashMap<String, Vec<usize>>,
}

impl ParamStack {
    pub(crate) fn new() -> Self {
        ParamStack {
            stacks: HashMap::new(),
        }
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.stacks.contains_key(name)
    }

    pub(crate) fn add_entry(&mut self, name: &str) {
        // ignores the original entry even if it existed
        self.stacks.insert(name.to_string(), Vec::new());
    }

    pub(crate) fn get_value(&self, name: &str) -> Option<&usize> {
        self.stacks.get(name).and_then(|stack| stack.last())
    }

    pub(crate) fn push_value(&mut self, name: &str, value: usize) -> Option<()> {
        self.stacks.get_mut(name).map(|stack| stack.push(value))
    }

    pub(crate) fn pop_value(&mut self, name: &str) -> Option<usize> {
        self.stacks.get_mut(name).and_then(|stack| stack.pop())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SchemaParseError {
    UnexpectedEof,
    UnexpectedToken,
    UnknownToken,
    UnknownError(String),
}

impl std::fmt::Display for SchemaParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error in parsing schema")
    }
}

impl std::error::Error for SchemaParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let input = "";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected = Err(SchemaParseError::UnexpectedEof);

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_single_field() {
        let input = "fld1:INT16";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected_ast = Ast {
            name: "".to_owned(),
            kind: AstKind::Struct(vec![Ast {
                name: "fld1".to_owned(),
                kind: AstKind::Int16,
            }]),
        };
        let expected = Ok(Schema {
            ast: expected_ast,
            params: ParamStack::new(),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_single_struct() {
        let input = "fld1:[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected_ast = Ast {
            name: "".to_owned(),
            kind: AstKind::Struct(vec![Ast {
                name: "fld1".to_owned(),
                kind: AstKind::Struct(vec![
                    Ast {
                        name: "sfld1".to_owned(),
                        kind: AstKind::NStr(4),
                    },
                    Ast {
                        name: "sfld2".to_owned(),
                        kind: AstKind::Str,
                    },
                    Ast {
                        name: "sfld3".to_owned(),
                        kind: AstKind::Int32,
                    },
                ]),
            }]),
        };
        let expected = Ok(Schema {
            ast: expected_ast,
            params: ParamStack::new(),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_nested_struct() {
        let input = "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]]";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected_ast = Ast {
            name: "".to_owned(),
            kind: AstKind::Struct(vec![Ast {
                name: "fld1".to_owned(),
                kind: AstKind::Struct(vec![Ast {
                    name: "sfld1".to_owned(),
                    kind: AstKind::Struct(vec![
                        Ast {
                            name: "ssfld1".to_owned(),
                            kind: AstKind::NStr(4),
                        },
                        Ast {
                            name: "ssfld2".to_owned(),
                            kind: AstKind::Str,
                        },
                        Ast {
                            name: "ssfld3".to_owned(),
                            kind: AstKind::Int32,
                        },
                    ]),
                }]),
            }]),
        };
        let expected = Ok(Schema {
            ast: expected_ast,
            params: ParamStack::new(),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_single_fixed_length_array() {
        let input = "fld1:{3}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected_ast = Ast {
            name: "".to_owned(),
            kind: AstKind::Struct(vec![Ast {
                name: "fld1".to_owned(),
                kind: AstKind::Array(
                    Len::Fixed(3),
                    Box::new(Ast {
                        name: "[]".to_owned(),
                        kind: AstKind::Struct(vec![
                            Ast {
                                name: "sfld1".to_owned(),
                                kind: AstKind::NStr(4),
                            },
                            Ast {
                                name: "sfld2".to_owned(),
                                kind: AstKind::Str,
                            },
                            Ast {
                                name: "sfld3".to_owned(),
                                kind: AstKind::Int32,
                            },
                        ]),
                    }),
                ),
            }]),
        };
        let expected = Ok(Schema {
            ast: expected_ast,
            params: ParamStack::new(),
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_single_variable_length_array() {
        let input = "fld1:INT8,fld2:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected_ast = Ast {
            name: "".to_owned(),
            kind: AstKind::Struct(vec![
                Ast {
                    name: "fld1".to_owned(),
                    kind: AstKind::Int8,
                },
                Ast {
                    name: "fld2".to_owned(),
                    kind: AstKind::Array(
                        Len::Variable("fld1".to_owned()),
                        Box::new(Ast {
                            name: "[]".to_owned(),
                            kind: AstKind::Struct(vec![
                                Ast {
                                    name: "sfld1".to_owned(),
                                    kind: AstKind::NStr(4),
                                },
                                Ast {
                                    name: "sfld2".to_owned(),
                                    kind: AstKind::Str,
                                },
                                Ast {
                                    name: "sfld3".to_owned(),
                                    kind: AstKind::Int32,
                                },
                            ]),
                        }),
                    ),
                },
            ]),
        };
        let mut params = ParamStack::new();
        params.add_entry("fld1");

        let expected = Ok(Schema {
            ast: expected_ast,
            params,
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn lex() {
        let input = "fld1:INT16,fld2:[sfld1:INT16,sfld2:INT8],fld3:{3}[sfld1:INT16,sfld2:INT8]";
        let lexer = SchemaLexer::new(input.as_bytes());
        let actual = lexer.collect::<Vec<_>>();
        let expected = vec![
            Token::Ident("fld1".to_owned()),
            Token::Colon,
            Token::Ident("INT16".to_owned()),
            Token::Comma,
            Token::Ident("fld2".to_owned()),
            Token::Colon,
            Token::LBracket,
            Token::Ident("sfld1".to_owned()),
            Token::Colon,
            Token::Ident("INT16".to_owned()),
            Token::Comma,
            Token::Ident("sfld2".to_owned()),
            Token::Colon,
            Token::Ident("INT8".to_owned()),
            Token::RBracket,
            Token::Comma,
            Token::Ident("fld3".to_owned()),
            Token::Colon,
            Token::LBrace,
            Token::Number(3),
            Token::RBrace,
            Token::LBracket,
            Token::Ident("sfld1".to_owned()),
            Token::Colon,
            Token::Ident("INT16".to_owned()),
            Token::Comma,
            Token::Ident("sfld2".to_owned()),
            Token::Colon,
            Token::Ident("INT8".to_owned()),
            Token::RBracket,
        ];
        let expected = expected.iter().map(|t| Ok(t.clone())).collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn lex_empty() {
        let input = "";
        let lexer = SchemaLexer::new(input.as_bytes());
        let actual = lexer.collect::<Vec<_>>();
        assert_eq!(actual, Vec::<Result<Token, SchemaParseError>>::new());
    }
}
