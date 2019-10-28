//
// Copyright 2006 Anthony J. Lorelli
// $Id: Parser.cs 282 2006-03-12 16:14:23Z  $
//
namespace Scheme {
    using System.Text;

    public class Parser {
        private Scanner _s;
        
        public Parser(Scanner s) { _s = s; }

        public object Read() {
            return _Read(_s.NextToken());
        }

        private object _Read(Token t) {
            switch (t.Type) {
                case TokenType.Boolean:
                    return Boolean.Create(t.Text);
                case TokenType.Character:
                    return new Character(t.Text[0]);
                case TokenType.EOF:
                    return null;
                case TokenType.Integer:
                    try {
                        return long.Parse(t.Text);
                    } catch (System.FormatException) {
                        throw new System.Exception(
                            "Invalid integer format: " + t.Text
                        );
                    }
                case TokenType.LeftParen:
                    return ReadList();
                case TokenType.Quote:
                    object[] quoteArr = new object[2];
                    quoteArr[0] = SymbolCache.QUOTE;
                    quoteArr[1] = Read();
                    return List.Create(quoteArr);
                case TokenType.Quasiquote:
                    object[] qquoteArr = new object[2];
                    qquoteArr[0] = SymbolCache.QUASIQUOTE;
                    qquoteArr[1] = Read();
                    return List.Create(qquoteArr);
                case TokenType.Unquote:
                    object[] unquoteArr = new object[2];
                    unquoteArr[0] = SymbolCache.UNQUOTE;
                    unquoteArr[1] = Read();
                    return List.Create(unquoteArr);
                case TokenType.UnquoteSplicing:
                    object[] unqspliceArr = new object[2];
                    unqspliceArr[0] = SymbolCache.UNQUOTESPLICING;
                    unqspliceArr[1] = Read();
                    return List.Create(unqspliceArr);
                case TokenType.RightParen:
                    throw new System.Exception("Unmatched right parenthesis.");
                case TokenType.Dot:
                    throw new System.Exception("Unrecognized use of dot notation");
                case TokenType.String:
                    return t.Text;
                case TokenType.Symbol:
                    return SymbolFactory.Create(t.Text);
                case TokenType.Vector:
                    Token nt = _s.NextToken();
                    if (nt.Type == TokenType.LeftParen) {
                        return List.ToVector((Pair)ReadList());
                    } else {
                        throw new System.Exception("Invalid vector literal");
                    }
                default:
                    throw new System.Exception("Unrecognized token");
            }
        }

        private object ReadList() {
            Token t = _s.NextToken();
            switch (t.Type) {
                case TokenType.RightParen:
                    return Pair.EmptyList;
                case TokenType.Dot:
                    object o = Read();
                    Token nt = _s.NextToken();
                    if (nt.Type != TokenType.RightParen) {
                        throw new System.Exception("Missing close parenthesis after " + o);
                    } else {
                        return o;
                    }
                default:
                    return new Pair(_Read(t), ReadList());
            }
        }
    }
}
