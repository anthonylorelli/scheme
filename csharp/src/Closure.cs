//
// Copyright 2006 Anthony J. Lorelli
// $Id: Closure.cs 303 2006-03-22 22:44:57Z  $
//
namespace Scheme {
    public class Closure : Procedure {
        private Pair _args;
        private Pair _exp;
        private Environment _env;
        private bool _variableArity;
        private long _argListLength;

        public Closure(object args, Pair exp, Environment env) { 
            if (args is Symbol) {
                _variableArity = true;
                _args = new Pair(args, Pair.EmptyList);
            } else {
                Pair pairArgs = (Pair)args;
                _variableArity = !List.IsProper(pairArgs);
                _args = (_variableArity) ? _MakeArgList(pairArgs) : pairArgs;
            }
            _argListLength = List.Length(_args);
            _exp = exp; 
            _env = env; 
        }

        private static Pair _MakeArgList(Pair args) {
            return (args == Pair.EmptyList) ? args :
                new Pair(args.Head, 
                    (args.Tail is Pair) ? 
                        _MakeArgList((Pair)args.Tail) : 
                        new Pair(args.Tail, Pair.EmptyList));
        }

        public Pair Body {
            get { return _exp; }
        }

        public Environment BindArguments(Pair args) {
            Pair p = (_variableArity) ? _SeparateValues(args, 0) : args;
            return new Environment(_args, p, _env);
        }

        public object Apply(Interpreter i, Pair args, Environment env) {
            Environment cenv = BindArguments(args);

            return i.Execute(_exp, Pair.EmptyList, cenv);
        }

        private Pair _SeparateValues(Pair args, int depth) {
            return (depth < _argListLength - 1) ?
                new Pair(args.Head, 
                    _SeparateValues((Pair)args.Tail, depth + 1)) :
                new Pair(args, Pair.EmptyList);
        }

        public override string ToString() {
            string s = "<#function>";
            s += " Args: " + _args;
            //s += "\nBody: " + _exp;
            return s;
        }
    }
}
