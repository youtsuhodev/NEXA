//! Lexical analysis for NEXA source.

use nexa_errors::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Fn,
    Return,
    Int,
    String,
    Void,
    Ident(String),
    IntLit(i64),
    StringLit(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Arrow,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token()?;
            let eof = matches!(t.kind, TokenKind::Eof);
            out.push(t);
            if eof {
                break;
            }
        }
        Ok(out)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_ws();
        let start = self.pos as u32;
        let Some(c) = self.peek_char() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(start, start),
            });
        };

        match c {
            '+' => {
                self.bump();
                Ok(self.tok(TokenKind::Plus, start))
            }
            '-' => {
                if self.peek_next() == Some('>') {
                    self.bump();
                    self.bump();
                    Ok(Token {
                        kind: TokenKind::Arrow,
                        span: Span::new(start, self.pos as u32),
                    })
                } else {
                    self.bump();
                    Ok(self.tok(TokenKind::Minus, start))
                }
            }
            '*' => {
                self.bump();
                Ok(self.tok(TokenKind::Star, start))
            }
            '/' => {
                self.bump();
                Ok(self.tok(TokenKind::Slash, start))
            }
            '(' => {
                self.bump();
                Ok(self.tok(TokenKind::LParen, start))
            }
            ')' => {
                self.bump();
                Ok(self.tok(TokenKind::RParen, start))
            }
            '{' => {
                self.bump();
                Ok(self.tok(TokenKind::LBrace, start))
            }
            '}' => {
                self.bump();
                Ok(self.tok(TokenKind::RBrace, start))
            }
            ',' => {
                self.bump();
                Ok(self.tok(TokenKind::Comma, start))
            }
            ';' => {
                self.bump();
                Ok(self.tok(TokenKind::Semicolon, start))
            }
            ':' => {
                self.bump();
                Ok(self.tok(TokenKind::Colon, start))
            }
            '"' => self.string_lit(start),
            '0'..='9' => self.int_lit(start),
            'a'..='z' | 'A'..='Z' | '_' => self.ident_or_kw(start),
            _ => Err(format!(
                "unexpected character {:?} at byte {}",
                c, self.pos
            )),
        }
    }

    fn tok(&self, kind: TokenKind, start: u32) -> Token {
        Token {
            kind,
            span: Span::new(start, self.pos as u32),
        }
    }

    fn ident_or_kw(&mut self, start: u32) -> Result<Token, String> {
        self.bump();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
        let s = &self.src[start as usize..self.pos];
        let kind = match s {
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "Int" => TokenKind::Int,
            "String" => TokenKind::String,
            "Void" => TokenKind::Void,
            _ => TokenKind::Ident(s.to_string()),
        };
        Ok(Token {
            kind,
            span: Span::new(start, self.pos as u32),
        })
    }

    fn int_lit(&mut self, start: u32) -> Result<Token, String> {
        self.bump();
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        let s = &self.src[start as usize..self.pos];
        let n: i64 = s
            .parse()
            .map_err(|_| format!("invalid integer literal {:?}", s))?;
        Ok(Token {
            kind: TokenKind::IntLit(n),
            span: Span::new(start, self.pos as u32),
        })
    }

    fn string_lit(&mut self, start: u32) -> Result<Token, String> {
        self.bump(); // opening "
        let mut buf = String::new();
        loop {
            let Some(c) = self.peek_char() else {
                return Err("unterminated string literal".into());
            };
            if c == '"' {
                self.bump();
                break;
            }
            if c == '\\' {
                self.bump();
                let Some(esc) = self.peek_char() else {
                    return Err("unterminated escape in string".into());
                };
                match esc {
                    'n' => {
                        buf.push('\n');
                        self.bump();
                    }
                    't' => {
                        buf.push('\t');
                        self.bump();
                    }
                    '"' => {
                        buf.push('"');
                        self.bump();
                    }
                    '\\' => {
                        buf.push('\\');
                        self.bump();
                    }
                    _ => return Err(format!("unknown escape \\{}", esc)),
                }
            } else {
                self.bump();
                buf.push(c);
            }
        }
        Ok(Token {
            kind: TokenKind::StringLit(buf),
            span: Span::new(start, self.pos as u32),
        })
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_ascii_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) {
        let Some(c) = self.peek_char() else {
            return;
        };
        self.pos += c.len_utf8();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_fn_main() {
        let src = r#"fn main() { print("hi"); }"#;
        let kinds: Vec<_> = Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect();
        assert!(matches!(kinds.first(), Some(TokenKind::Fn)));
        assert!(kinds.iter().any(|k| matches!(k, TokenKind::Ident(s) if s == "print")));
    }
}
