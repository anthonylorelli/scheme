//
// Copyright 2006 Anthony J. Lorelli
// $Id: Token.cs 282 2006-03-12 16:14:23Z  $
//
namespace Scheme {
    public enum TokenType {
        Boolean,
        Character,
        Dot,
        EOF,
        Integer,
        LeftParen,
        Quasiquote,
        Quote,
        RightParen,
        String,
        Symbol,
        Unquote,
        UnquoteSplicing,
        Vector
    }

    public class Token {
        private readonly string _text;
        private readonly TokenType _type;

        public Token(string text, TokenType type) {
            _text = text; _type = type;
        }

        public TokenType Type {
            get { return _type; }
        }

        public string Text {
            get { return _text; }
        }
    }
}
