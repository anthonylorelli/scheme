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
                    int firstCh = GetChar();
                    if (firstCh == -1) {
                        throw new System.Exception("Error: unexpected EOF in character literal");
                    }
                    char fc = (char)firstCh;
                    if (char.IsWhiteSpace(fc) || fc == '(' || fc == ')' ||
                        fc == '\'' || fc == '"' || fc == ';') {
                        return new Token(fc.ToString(), TokenType.Character);
                    }
                    StringBuilder charBuf = new StringBuilder();
                    charBuf.Append(fc);
                    int nextCh = GetChar();
                    while (nextCh != -1 && !char.IsWhiteSpace((char)nextCh) &&
                           nextCh != '(' && nextCh != ')' && nextCh != '\'' &&
                           nextCh != '"' && nextCh != ';') {
                        charBuf.Append((char)nextCh);
                        nextCh = GetChar();
                    }
                    if (nextCh != -1) PushChar(nextCh);
                    string charText = charBuf.ToString();
                    if (charText.Length == 1) {
                        return new Token(charText, TokenType.Character);
                    } else {
                        string s = charText.ToLower();
                        if (s == "space") {
                            return new Token(" ", TokenType.Character);
                        } else if (s == "newline") {
                            return new Token("\n", TokenType.Character);
                        } else {
                            throw new System.Exception(
                                "Error: unrecognized #\\... literal: " +
                                "#\\" + charText
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

                    while ((ch = _reader.Read()) != -1 && char.IsDigit((char)ch)) {
                        strBuffer.Append((char)ch);
                    }

                    if (ch == -1 || char.IsWhiteSpace((char)ch) || ch == ')' ||
                        ch == '(' || ch == '\'' || ch == '"' || ch == ';') {
                        if (ch != -1) PushChar(ch);
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
                        ch != -1 && !char.IsWhiteSpace((char)ch) && ch != '(' && 
                        ch != ')' && ch != '\'' && ch != '"' && ch != ';'
                    );

                    if (ch != -1) PushChar(ch);

                    string raw = strBuffer.ToString();
                    if (raw.Length > 1 && (raw[0] == '-' || raw[0] == '+')) {
                        bool allDigits = true;
                        for (int idx = 1; idx < raw.Length; ++idx) {
                            if (!char.IsDigit(raw[idx])) {
                                allDigits = false;
                                break;
                            }
                        }
                        if (allDigits) {
                            string numText = (raw[0] == '+') ? raw.Substring(1) : raw;
                            return new Token(numText, TokenType.Integer);
                        }
                    }

                    string s = raw.ToLower();

                    return new Token(s,
                        (s == ".") ? TokenType.Dot : TokenType.Symbol);
            }
        }
    }
}
