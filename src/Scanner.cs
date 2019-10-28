//
// Copyright 2006 Anthony J. Lorelli
// $Id: Scanner.cs 282 2006-03-12 16:14:23Z  $
//
namespace Scheme {
    using System.IO;
    using System.Text;

    public class Scanner {
        private TextReader _reader;
        private int _ch;
        private bool _haveChar;

        public Scanner(TextReader reader) {
            _reader = reader; 
            _haveChar = false;
        }

        private int PopChar() {
            _haveChar = false;
            return _ch;
        }

        private void PushChar(int ch) {
            _ch = ch; _haveChar = true;
        }

        private int GetChar() {
            return _haveChar ? PopChar() : _reader.Read();
        }

        private Token PoundLiteral() {
            int ch = GetChar();
            switch (ch) {
                case 't': case 'T':
                    return new Token("#t", TokenType.Boolean);
                case 'f': case 'F':
                    return new Token("#f", TokenType.Boolean);
                case '(':
                    PushChar(ch);
                    return new Token("#", TokenType.Vector);
                case '\\':
                    Token t = NextToken();
                    if (t.Text.Length == 1) {
                        return new Token(t.Text, TokenType.Character);
                    } else {
                        string s = t.Text.ToLower();
                        if (s == "space") {
                            return new Token(" ", TokenType.Character);
                        } else if (s == "newline") {
                            return new Token("\n", TokenType.Character);
                        } else {
                            throw new System.Exception(
                                "Error: unrecognized #\\... literal: " +
                                "#\\" + t.Text
                            );
                        }
                    }
                default:
                    throw new System.Exception(
                        "Error: badly formed #... literal: " + "#" + (char)ch
                    ); 
            }
        }

        private Token StringLiteral() {
            StringBuilder strBuffer = new StringBuilder();

            int ch;
            while ((ch = _reader.Read()) != '"' && ch != -1) {
                if ( ch != '\\') strBuffer.Append((char)ch);
            }

            return new Token(strBuffer.ToString(), TokenType.String);
        }

        public Token NextToken() {
            StringBuilder strBuffer = new StringBuilder();
            int ch = GetChar();

            while (char.IsWhiteSpace((char)ch)) ch = _reader.Read();

            switch (ch) {
                case -1:
                    return new Token("", TokenType.EOF);
                case '(':
                    return new Token("(", TokenType.LeftParen);
                case ')':
                    return new Token(")", TokenType.RightParen);
                case '\'':
                    return new Token("'", TokenType.Quote);
                case '`':
                    return new Token("`", TokenType.Quasiquote);
                case ',':
                    ch = _reader.Read();
                    if (ch == '@') {
                        return new Token(",@", TokenType.UnquoteSplicing);
                    } else {
                        PushChar(ch);
                        return new Token(",", TokenType.Unquote);
                    }
                case '"':
                    return StringLiteral();
                case '#':
                    return PoundLiteral();
                case ';':
                    _reader.ReadLine();
                    return NextToken();
                case '0': case '1': case '2': case '3': case '4':
                case '5': case '6': case '7': case '8': case '9':
                    strBuffer.Append((char)ch);

                    while (char.IsDigit((char)(ch = _reader.Read()))) {
                        strBuffer.Append((char)ch);
                    }

                    if (char.IsWhiteSpace((char)ch) || ch == ')') {
                        PushChar(ch);
                        return new Token(strBuffer.ToString(), TokenType.Integer);
                    } else {
                        strBuffer.Append((char)ch);
                        throw new System.Exception(
                            "Error: unrecognized lexical sequence: " +
                            strBuffer.ToString()
                        );
                    }
                default:
                    do {
                        strBuffer.Append((char)ch);
                        ch = _reader.Read();
                    } while (
                        !char.IsWhiteSpace((char)ch) && ch != -1 && ch != '(' && 
                        ch != ')' && ch != '\'' && ch != '"' && ch != ';'
                    );

                    PushChar(ch);

                    string s = strBuffer.ToString().ToLower();

                    return new Token(s,
                        (s == ".") ? TokenType.Dot : TokenType.Symbol);
            }
        }
    }
}
