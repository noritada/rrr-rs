use crate::param::ParamStack;
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

// after running self.lexer.next(), self.pos must be updated accordingly
struct SchemaParser<'b> {
    lexer: std::iter::Peekable<SchemaLexer<'b>>,
    pos: usize,
    params: ParamStack,
}

impl<'b> SchemaParser<'b> {
    fn new(input: &'b [u8]) -> Self {
        Self {
            lexer: SchemaLexer::new(input).peekable(),
            pos: 0,
            params: ParamStack::new(),
        }
    }

    fn parse(mut self) -> Result<Schema, SchemaParseError> {
        let kind = self.parse_field_list()?;
        if let Some(result) = self.lexer.next() {
            // should be TokenKind::RBracket
            let token = result.unwrap();
            self.pos = token.pos;
            return Err(SchemaParseError {
                kind: SchemaParseErrorKind::UnknownError(
                    "reading field list finished but some tokens are unexpectedly left".to_owned(),
                ),
                pos: self.pos,
            });
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
            let Token { kind, pos } = token?;
            self.pos = pos;
            let name = if let TokenKind::Ident(s) = kind {
                s
            } else {
                return Err(self.err_unexpected_token());
            };

            self.consume_symbol(TokenKind::Colon)?;

            let kind = self.parse_type()?;
            let member = Ast { kind, name };
            members.push(member);

            if matches!(
                self.lexer.peek(),
                None | Some(Ok(Token {
                    kind: TokenKind::RBracket,
                    ..
                }))
            ) {
                break;
            }

            // actually EOF has been captured in the previous block
            if self.next_token()?.kind != TokenKind::Comma {
                return Err(self.err_unexpected_token());
            }
        }

        if members.is_empty() {
            return Err(self.err_unexpected_eof());
        }

        let kind = AstKind::Struct(members);
        Ok(kind)
    }

    fn parse_type(&mut self) -> Result<AstKind, SchemaParseError> {
        match self.next_token()?.kind {
            TokenKind::Ident(s) => self.parse_builtin_type(s),
            TokenKind::LBracket => {
                let kind = self.parse_field_list()?;
                // no tokens other than TokenKind::RBracket or EOF appears
                self.consume_next_token()?;
                Ok(kind)
            }
            TokenKind::LAngleBracket => self.parse_nstr_type(),
            TokenKind::LBrace => self.parse_array(),
            _ => Err(self.err_unexpected_token()),
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
                return Err(SchemaParseError {
                    kind: SchemaParseErrorKind::UnknownError(format!(
                        "unknown builtin type {ident}"
                    )),
                    pos: self.pos,
                })
            }
        };
        Ok(kind)
    }

    fn parse_nstr_type(&mut self) -> Result<AstKind, SchemaParseError> {
        // LAngleBracket has already been read
        let len = self.consume_number()?;
        self.consume_symbol(TokenKind::RAngleBracket)?;

        if let TokenKind::Ident(s) = self.next_token()?.kind {
            if s.as_str() != "NSTR" {
                return Err(self.err_unexpected_token());
            }
        } else {
            return Err(self.err_unexpected_token());
        }

        let kind = AstKind::NStr(len);
        Ok(kind)
    }

    fn parse_array(&mut self) -> Result<AstKind, SchemaParseError> {
        // LBrace has already been read
        let len = match self.next_token()?.kind {
            TokenKind::Number(n) => Len::Fixed(n),
            TokenKind::Ident(s) => {
                self.params.add_entry(&s);
                Len::Variable(s)
            }
            _ => return Err(self.err_unexpected_token()),
        };

        self.consume_symbol(TokenKind::RBrace)?;
        self.consume_symbol(TokenKind::LBracket)?;
        let struct_kind = self.parse_field_list()?;
        // no tokens other than TokenKind::RBracket or EOF appears
        self.consume_next_token()?;

        let struct_node = Ast {
            kind: struct_kind,
            name: "[]".to_owned(),
        };
        Ok(AstKind::Array(len, Box::new(struct_node)))
    }

    fn consume_number(&mut self) -> Result<usize, SchemaParseError> {
        match self.next_token()?.kind {
            TokenKind::Number(n) => Ok(n),
            _ => Err(self.err_unexpected_token()),
        }
    }

    fn consume_symbol(&mut self, symbol: TokenKind) -> Result<(), SchemaParseError> {
        if self.next_token()?.kind != symbol {
            return Err(self.err_unexpected_token());
        }
        Ok(())
    }

    fn next_token(&mut self) -> Result<Token, SchemaParseError> {
        let token = self
            .lexer
            .next()
            .unwrap_or(Err(self.err_unexpected_eof()))?;
        self.pos = token.pos;
        Ok(token)
    }

    fn consume_next_token(&mut self) -> Result<(), SchemaParseError> {
        match self.lexer.next() {
            Some(Ok(token)) => {
                self.pos = token.pos;
                Ok(())
            }
            None => Err(self.err_unexpected_eof()),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn err_unexpected_eof(&self) -> SchemaParseError {
        SchemaParseError::unexpected_eof(self.pos)
    }

    #[inline]
    fn err_unexpected_token(&self) -> SchemaParseError {
        SchemaParseError::unexpected_token(self.pos)
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
        let kind =
            TokenKind::Ident(String::from_utf8_lossy(&self.input[start..self.pos]).to_string());
        Token::new(kind, start)
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && matches!(self.input[self.pos], b'0'..=b'9') {
            self.pos += 1;
        }
        let kind = TokenKind::Number(
            (String::from_utf8_lossy(&self.input[start..self.pos]).parse()).unwrap(),
        );
        Token::new(kind, start)
    }
}

impl<'b> Iterator for SchemaLexer<'b> {
    type Item = Result<Token, SchemaParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        macro_rules! lex {
            ($kind:expr) => {{
                let pos = self.pos;
                self.pos += 1;
                Ok(Token::new($kind, pos))
            }};
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let token = match self.input[self.pos] {
            b'A'..=b'Z' | b'a'..=b'z' => Ok(self.lex_ident()),
            b'1'..=b'9' => Ok(self.lex_number()),
            b':' => lex!(TokenKind::Colon),
            b',' => lex!(TokenKind::Comma),
            b'[' => lex!(TokenKind::LBracket),
            b']' => lex!(TokenKind::RBracket),
            b'<' => lex!(TokenKind::LAngleBracket),
            b'>' => lex!(TokenKind::RAngleBracket),
            b'{' => lex!(TokenKind::LBrace),
            b'}' => lex!(TokenKind::RBrace),
            _ => Err(SchemaParseError {
                kind: SchemaParseErrorKind::UnknownToken,
                pos: self.pos,
            }),
        };
        Some(token)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.input.len()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    pos: usize,
}

impl Token {
    fn new(kind: TokenKind, pos: usize) -> Token {
        Token { kind, pos }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
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
pub struct SchemaParseError {
    kind: SchemaParseErrorKind,
    pos: usize,
}

impl SchemaParseError {
    #[inline]
    fn unexpected_eof(pos: usize) -> Self {
        Self {
            kind: SchemaParseErrorKind::UnexpectedEof,
            pos,
        }
    }

    #[inline]
    fn unexpected_token(pos: usize) -> Self {
        Self {
            kind: SchemaParseErrorKind::UnexpectedToken,
            pos,
        }
    }
}

impl std::fmt::Display for SchemaParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error in parsing schema")
    }
}

impl std::error::Error for SchemaParseError {}

#[derive(Debug, PartialEq, Eq)]
pub enum SchemaParseErrorKind {
    UnexpectedEof,
    UnexpectedToken,
    UnknownToken,
    UnknownError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let input = "";
        let parser = SchemaParser::new(input.as_bytes());
        let actual = parser.parse();
        let expected = Err(SchemaParseError::unexpected_eof(0));

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
            (TokenKind::Ident("fld1".to_owned()), 0),
            (TokenKind::Colon, 4),
            (TokenKind::Ident("INT16".to_owned()), 5),
            (TokenKind::Comma, 10),
            (TokenKind::Ident("fld2".to_owned()), 11),
            (TokenKind::Colon, 15),
            (TokenKind::LBracket, 16),
            (TokenKind::Ident("sfld1".to_owned()), 17),
            (TokenKind::Colon, 22),
            (TokenKind::Ident("INT16".to_owned()), 23),
            (TokenKind::Comma, 28),
            (TokenKind::Ident("sfld2".to_owned()), 29),
            (TokenKind::Colon, 34),
            (TokenKind::Ident("INT8".to_owned()), 35),
            (TokenKind::RBracket, 39),
            (TokenKind::Comma, 40),
            (TokenKind::Ident("fld3".to_owned()), 41),
            (TokenKind::Colon, 45),
            (TokenKind::LBrace, 46),
            (TokenKind::Number(3), 47),
            (TokenKind::RBrace, 48),
            (TokenKind::LBracket, 49),
            (TokenKind::Ident("sfld1".to_owned()), 50),
            (TokenKind::Colon, 55),
            (TokenKind::Ident("INT16".to_owned()), 56),
            (TokenKind::Comma, 61),
            (TokenKind::Ident("sfld2".to_owned()), 62),
            (TokenKind::Colon, 67),
            (TokenKind::Ident("INT8".to_owned()), 68),
            (TokenKind::RBracket, 72),
        ];
        let expected = expected
            .iter()
            .map(|(kind, pos)| {
                Ok(Token {
                    kind: kind.clone(),
                    pos: *pos,
                })
            })
            .collect::<Vec<_>>();
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
