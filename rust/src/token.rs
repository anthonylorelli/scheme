#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Boolean,
    Character,
    Dot,
    Eof,
    Integer,
    LeftParen,
    Quasiquote,
    Quote,
    RightParen,
    String,
    Symbol,
    Unquote,
    UnquoteSplicing,
    Vector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub kind: TokenType,
}

impl Token {
    pub fn new(text: impl Into<String>, kind: TokenType) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}
