use std::cell::RefCell;
use std::io::BufRead;
use std::rc::Rc;

use crate::scanner::Scanner;
use crate::symbol::SymbolTable;
use crate::token::{Token, TokenType};
use crate::value::Value;

pub struct Parser<R: BufRead> {
    scanner: Scanner<R>,
    symbols: Rc<RefCell<SymbolTable>>,
}

impl<R: BufRead> Parser<R> {
    pub fn new(scanner: Scanner<R>, symbols: Rc<RefCell<SymbolTable>>) -> Self {
        Self { scanner, symbols }
    }

    pub fn read(&mut self) -> Result<Option<Value>, String> {
        let token = self.scanner.next_token()?;
        if token.kind == TokenType::Eof {
            Ok(None)
        } else {
            Ok(Some(self.read_from_token(token)?))
        }
    }

    fn read_required(&mut self, context: &str) -> Result<Value, String> {
        match self.read()? {
            Some(v) => Ok(v),
            None => Err(format!("Error: unexpected EOF ({context})")),
        }
    }

    fn read_from_token(&mut self, t: Token) -> Result<Value, String> {
        match t.kind {
            TokenType::Boolean => Ok(Value::Bool(t.text.eq_ignore_ascii_case("#t"))),
            TokenType::Character => {
                let ch = t
                    .text
                    .chars()
                    .next()
                    .ok_or_else(|| "Invalid empty character token".to_string())?;
                Ok(Value::Char(ch))
            }
            TokenType::Eof => Ok(Value::Void),
            TokenType::Integer => {
                let n = t
                    .text
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid integer format: {}", t.text))?;
                Ok(Value::Int(n))
            }
            TokenType::LeftParen => self.read_list(),
            TokenType::Quote => {
                let sym = self.symbols.borrow().cache.quote.clone();
                let datum = self.read_required("after quote")?;
                Ok(Value::list_from_vec(vec![Value::Symbol(sym), datum]))
            }
            TokenType::Quasiquote => {
                let sym = self.symbols.borrow().cache.quasiquote.clone();
                let datum = self.read_required("after quasiquote")?;
                Ok(Value::list_from_vec(vec![Value::Symbol(sym), datum]))
            }
            TokenType::Unquote => {
                let sym = self.symbols.borrow().cache.unquote.clone();
                let datum = self.read_required("after unquote")?;
                Ok(Value::list_from_vec(vec![Value::Symbol(sym), datum]))
            }
            TokenType::UnquoteSplicing => {
                let sym = self.symbols.borrow().cache.unquote_splicing.clone();
                let datum = self.read_required("after unquote-splicing")?;
                Ok(Value::list_from_vec(vec![Value::Symbol(sym), datum]))
            }
            TokenType::RightParen => Err("Unmatched right parenthesis.".to_string()),
            TokenType::Dot => Err("Unrecognized use of dot notation".to_string()),
            TokenType::String => Ok(Value::Str(Rc::new(t.text))),
            TokenType::Symbol => {
                let sym = self.symbols.borrow_mut().intern(&t.text);
                Ok(Value::Symbol(sym))
            }
            TokenType::Vector => {
                let nt = self.scanner.next_token()?;
                if nt.kind == TokenType::LeftParen {
                    let list = self.read_list()?;
                    let vec = list.to_proper_vec()?;
                    Ok(Value::Vector(Rc::new(RefCell::new(vec))))
                } else {
                    Err("Invalid vector literal".to_string())
                }
            }
        }
    }

    fn read_list(&mut self) -> Result<Value, String> {
        let mut items = Vec::new();
        loop {
            let t = self.scanner.next_token()?;
            match t.kind {
                TokenType::RightParen => {
                    return Ok(Value::list_from_vec(items));
                }
                TokenType::Dot => {
                    if items.is_empty() {
                        return Err("Unrecognized use of dot notation".to_string());
                    }
                    let tail = self.read_required("after '.' in dotted pair")?;
                    let close = self.scanner.next_token()?;
                    if close.kind != TokenType::RightParen {
                        return Err(format!("Missing close parenthesis after {tail}"));
                    }
                    let mut acc = tail;
                    for item in items.into_iter().rev() {
                        acc = Value::cons(item, acc);
                    }
                    return Ok(acc);
                }
                TokenType::Eof => {
                    return Err("Unexpected EOF while reading list".to_string());
                }
                _ => {
                    items.push(self.read_from_token(t)?);
                }
            }
        }
    }
}
