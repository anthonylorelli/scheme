//
// Copyright 2006 Anthony J. Lorelli
// $Id: MacroExpander.cs 301 2006-03-21 22:32:35Z  $
//
namespace Scheme {
    public class MacroExpander {
        private Parser _parser;
        private Interpreter _i;

        public MacroExpander(Parser parser, Interpreter i) {
            _parser = parser;
            _i = i; 
        }

        public object Expand() {
            object expr = _parser.Read();

            object expander = _i.GlobalEnvironment.Lookup(SymbolCache.EXPANDER);
            Procedure p = expander as Procedure;

            if (expr != null && p != null) {
//                _i.Context = new Frame(_i.Context);
//                _i.Context.Expr = InstructionCache.Apply;
//                _i.Context.ValueRib = new Pair(expr, Pair.EmptyList);
//                _i.Context.Value = expander;
                return p.Apply(_i, new Pair(expr, Pair.EmptyList), null);
            } else {
                return expr;
            }
        }
    }
}
