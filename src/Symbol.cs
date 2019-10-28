//
// Copyright 2006 Anthony J. Lorelli
// $Id: Symbol.cs 295 2006-03-18 23:04:06Z  $
//
namespace Scheme {
    using System.Collections;

    public class Symbol {
        private readonly string _str;
        private Hashtable _propTable;
        private readonly bool _isGenSym;

        public Symbol(string str) {
            _str = str; 
            _propTable = new Hashtable();
            _isGenSym = false;
        }

        public Symbol(string str, bool isGenSym) {
            _str = str;
            _propTable = new Hashtable();
            _isGenSym = isGenSym;
        }

        public bool IsGenSym() { return _isGenSym; }

        public override string ToString() { return _str; }

        public Hashtable Properties { get { return _propTable; } }
    }

    public class SymbolFactory {
        private static Hashtable _symTable;
        private static int _symbolNumber;
        private static readonly string _genSymPrefix = "__%sym";

        static SymbolFactory() {
            _symTable = new Hashtable();
            _symbolNumber = 0;
        }

        public static Symbol Create(string str) {
            if (_symTable.Contains(str)) {
                return (Symbol)_symTable[str];
            } else {
                Symbol s = new Symbol(str);
                _symTable[str] = s;
                return s;
            }
        }

        public static Symbol Create() {
            string uniqueName = _GenerateUniqueName();
            Symbol s = new Symbol(uniqueName, true);
            _symTable[uniqueName] = s;
            return s;
        }

        private static string _GenerateUniqueName() {
            string newSymbol = _genSymPrefix + _symbolNumber++;
            while (_symTable.Contains(newSymbol)) {
                newSymbol = _genSymPrefix + _symbolNumber++;
            }

            return newSymbol;
        }
    }

    public class SymbolCache {
        public static readonly Symbol IF;
        public static readonly Symbol SET;
        public static readonly Symbol DEFINE;
        public static readonly Symbol LAMBDA;
        public static readonly Symbol QUOTE;
        public static readonly Symbol LETREC;
        public static readonly Symbol BEGIN;
        public static readonly Symbol AND;
        public static readonly Symbol OR;
        public static readonly Symbol EXPANDER;
        public static readonly Symbol QUASIQUOTE;
        public static readonly Symbol UNQUOTE;
        public static readonly Symbol UNQUOTESPLICING;
        public static readonly Symbol CALLWITHVALUES;
        public static readonly Symbol CALLWITHCURRENTCONTINUATION;
        public static readonly Symbol CALLCC;

        static SymbolCache() {
            IF = SymbolFactory.Create("if");
            SET = SymbolFactory.Create("set!");
            DEFINE = SymbolFactory.Create("define");
            LAMBDA = SymbolFactory.Create("lambda");
            QUOTE = SymbolFactory.Create("quote");
            LETREC = SymbolFactory.Create("letrec");
            BEGIN = SymbolFactory.Create("begin");
            AND = SymbolFactory.Create("and");
            OR = SymbolFactory.Create("or");
            EXPANDER = SymbolFactory.Create("_expander");
            QUASIQUOTE = SymbolFactory.Create("quasiquote");
            UNQUOTE = SymbolFactory.Create("unquote");
            UNQUOTESPLICING = SymbolFactory.Create("unquote-splicing");
            CALLWITHVALUES = SymbolFactory.Create("call-with-values");
            CALLWITHCURRENTCONTINUATION = 
                SymbolFactory.Create("call-with-current-continuation");
            CALLCC = SymbolFactory.Create("call/cc");
        }
    }
}
