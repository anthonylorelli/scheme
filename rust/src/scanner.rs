use std::io::BufRead;

use crate::token::{Token, TokenType};

pub struct Scanner<R: BufRead> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    pushed: Option<char>,
}

impl<R: BufRead> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: Vec::new(),
            pos: 0,
            pushed: None,
        }
    }

    fn push_char(&mut self, ch: char) {
        self.pushed = Some(ch);
    }

    fn read_raw_char(&mut self) -> Result<Option<char>, String> {
        if self.pos >= self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            let n = self
                .reader
                .read_until(b'\n', &mut self.buf)
                .map_err(|e| format!("I/O error while scanning: {e}"))?;
            if n == 0 {
                return Ok(None);
            }
        }

        let rest = &self.buf[self.pos..];
        let s = std::str::from_utf8(rest)
            .map_err(|e| format!("Invalid UTF-8 in input stream: {e}"))?;
        let mut chars = s.chars();
        if let Some(ch) = chars.next() {
            self.pos += ch.len_utf8();
            Ok(Some(ch))
        } else {
            Ok(None)
        }
    }

    fn get_char(&mut self) -> Result<Option<char>, String> {
        if let Some(ch) = self.pushed.take() {
            Ok(Some(ch))
        } else {
            self.read_raw_char()
        }
    }

    fn skip_line(&mut self) -> Result<(), String> {
        while let Some(ch) = self.get_char()? {
            if ch == '\n' {
                break;
            }
        }
        Ok(())
    }

    fn is_delimiter(ch: char) -> bool {
        ch.is_whitespace() || matches!(ch, '(' | ')' | '\'' | '"' | ';')
    }

    fn pound_literal(&mut self) -> Result<Token, String> {
        let Some(ch) = self.get_char()? else {
            return Err("Error: unexpected EOF after #".to_string());
        };

        match ch {
            't' | 'T' => Ok(Token::new("#t", TokenType::Boolean)),
            'f' | 'F' => Ok(Token::new("#f", TokenType::Boolean)),
            '(' => {
                self.push_char('(');
                Ok(Token::new("#", TokenType::Vector))
            }
            '\\' => {
                let Some(first) = self.get_char()? else {
                    return Err("Error: unexpected EOF in character literal".to_string());
                };
                if Self::is_delimiter(first) {
                    return Ok(Token::new(first.to_string(), TokenType::Character));
                }
                let mut text = String::from(first);
                while let Some(next) = self.get_char()? {
                    if Self::is_delimiter(next) {
                        self.push_char(next);
                        break;
                    }
                    text.push(next);
                }
                if text.chars().count() == 1 {
                    Ok(Token::new(text, TokenType::Character))
                } else {
                    match text.to_ascii_lowercase().as_str() {
                        "space" => Ok(Token::new(" ", TokenType::Character)),
                        "newline" => Ok(Token::new("\n", TokenType::Character)),
                        _ => Err(format!("Error: unrecognized #\\... literal: #\\{text}")),
                    }
                }
            }
            _ => Err(format!("Error: badly formed #... literal: #{ch}")),
        }
    }

    fn string_literal(&mut self) -> Result<Token, String> {
        let mut out = String::new();
        while let Some(ch) = self.get_char()? {
            if ch == '"' {
                return Ok(Token::new(out, TokenType::String));
            } else if ch == '\\' {
                if let Some(escaped) = self.get_char()? {
                    match escaped {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        other => out.push(other),
                    }
                }
            } else {
                out.push(ch);
            }
        }
        Ok(Token::new(out, TokenType::String))
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        loop {
            let mut ch_opt = self.get_char()?;
            while let Some(c) = ch_opt {
                if c.is_whitespace() {
                    ch_opt = self.get_char()?;
                } else {
                    break;
                }
            }

            let Some(ch) = ch_opt else {
                return Ok(Token::new("", TokenType::Eof));
            };

            return match ch {
                '(' => Ok(Token::new("(", TokenType::LeftParen)),
                ')' => Ok(Token::new(")", TokenType::RightParen)),
                '\'' => Ok(Token::new("'", TokenType::Quote)),
                '`' => Ok(Token::new("`", TokenType::Quasiquote)),
                ',' => {
                    if let Some(next) = self.get_char()? {
                        if next == '@' {
                            return Ok(Token::new(",@", TokenType::UnquoteSplicing));
                        }
                        self.push_char(next);
                    }
                    Ok(Token::new(",", TokenType::Unquote))
                }
                '"' => self.string_literal(),
                '#' => self.pound_literal(),
                ';' => {
                    self.skip_line()?;
                    continue;
                }
                '0'..='9' => {
                    let mut buf = String::from(ch);
                    while let Some(next) = self.get_char()? {
                        if next.is_ascii_digit() {
                            buf.push(next);
                        } else if Self::is_delimiter(next) {
                            self.push_char(next);
                            return Ok(Token::new(buf, TokenType::Integer));
                        } else {
                            buf.push(next);
                            return Err(format!("Error: unrecognized lexical sequence: {buf}"));
                        }
                    }
                    Ok(Token::new(buf, TokenType::Integer))
                }
                _ => {
                    let mut raw = String::from(ch);
                    while let Some(next) = self.get_char()? {
                        if Self::is_delimiter(next) {
                            self.push_char(next);
                            break;
                        }
                        raw.push(next);
                    }

                    if raw.len() > 1 && (raw.starts_with('-') || raw.starts_with('+')) {
                        let rest = &raw[1..];
                        if rest.bytes().all(|b| b.is_ascii_digit()) {
                            let normalized = if raw.starts_with('+') { rest } else { &raw };
                            return Ok(Token::new(normalized, TokenType::Integer));
                        }
                    }

                    let sym = raw.to_ascii_lowercase();
                    let kind = if sym == "." {
                        TokenType::Dot
                    } else {
                        TokenType::Symbol
                    };
                    Ok(Token::new(sym, kind))
                }
            };
        }
    }
}
