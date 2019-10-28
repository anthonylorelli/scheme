//
// Copyright 2006 Anthony J. Lorelli
// $Id: StandardLibrary.cs 311 2006-03-27 22:55:15Z  $
//
namespace Scheme {
    public class ValueList {
        private Pair _valList;

        public ValueList(Pair valList) { _valList = valList; }

        public Pair Values { get { return _valList; } }
    }

    public class StandardLibrary {
        private Pair _syms;
        private Pair _vals;
        private static StandardLibrary _stdlib = null;

        private static Symbol[] _names = {
            SymbolFactory.Create("cons"), SymbolFactory.Create("car"), 
            SymbolFactory.Create("cdr"), SymbolFactory.Create("display"), 
            SymbolFactory.Create("integer->char"), SymbolFactory.Create("+"), 
            SymbolFactory.Create("-"), SymbolFactory.Create(">"), 
            SymbolFactory.Create("<"), SymbolFactory.Create("zero?"), 
            SymbolFactory.Create("load"),
            SymbolFactory.Create("vector"), SymbolFactory.Create("vector-ref"),
            SymbolFactory.Create("string-length"), SymbolFactory.Create("set-car!"),
            SymbolFactory.Create("set-cdr!"), SymbolFactory.Create("procedure?"),
            SymbolFactory.Create("symbol?"), SymbolFactory.Create("pair?"),
            SymbolFactory.Create("null?"), SymbolFactory.Create("eq?"),
            SymbolFactory.Create("eqv?"), SymbolFactory.Create("boolean?"),
            SymbolFactory.Create("vector?"), SymbolFactory.Create("void"),
            SymbolFactory.Create("putprop"), SymbolFactory.Create("getprop"),
            SymbolFactory.Create("remprop"), SymbolFactory.Create("values"),
            /* SymbolFactory.Create("call-with-values"), */SymbolFactory.Create("="),
            SymbolFactory.Create("vector-length"), SymbolFactory.Create("append"),
            SymbolFactory.Create("string?"), SymbolFactory.Create("string=?"),
            SymbolFactory.Create("apply"), SymbolFactory.Create("string"),
            SymbolFactory.Create("list"), SymbolFactory.Create("number?"),
            SymbolFactory.Create("_native-eval"), SymbolFactory.Create("gensym"),
            SymbolFactory.Create("gensym?"), SymbolFactory.Create("list->vector"),
            SymbolFactory.Create("make-vector"), SymbolFactory.Create("vector-set!"),
            SymbolFactory.Create("_get-ticks"), SymbolFactory.Create("/"),
            SymbolFactory.Create("call-with-current-continuation"),
            SymbolFactory.Create("_compile"), SymbolFactory.Create("char?")
        };

        private static Procedure[] _procs = {
            new Cons(), new Car(), new Cdr(), new Display(), new IntegerToChar(),
            new Add(), new Subtract(), new GreaterThan(), new LessThan(),
            new ZeroPred(), new Load(), new VectorCtor(), new VectorRef(),
            new StringLength(), new SetCar(), new SetCdr(), new ProcedurePred(),
            new SymbolPred(), new PairPred(), new NullPred(), new EqPred(),
            new EqvPred(), new BooleanPred(), new VectorPred(), new VoidProc(),
            new PutProp(), new GetProp(), new RemProp(), new ValuesProc(),
            new NumericEqualsPred(), new VectorLength(),
            new AppendProc(), new StringPred(), new StringEqualsPred(),
            new ApplyProc(), new StringProc(),
            new ListProc(), new NumberPred(), new NativeEval(), new GenSymProc(),
            new GenSymPred(), new ListToVector(), new MakeVector(), new VectorSet(),
            new GetTicksProc(), new DivProc(), new CallCC(), new CompileProc(),
            new CharPred()
        };

        private StandardLibrary() {
            _syms = List.Create(_names);
            _vals = List.Create(_procs);
        }

        public static StandardLibrary Instance() {
            if( _stdlib == null ) _stdlib = new StandardLibrary();
            return _stdlib;
        }

        public Pair Symbols {
            get { return _syms; }
        }

        public Pair Values {
            get { return _vals; }
        }
    }

    public class PutProp : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Symbol s = (Symbol)List.First(args);
            Symbol prop = (Symbol)List.Second(args);
            
            s.Properties[prop] = List.Third(args);

            return null;
        }
    }

    public class AppendProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return _Append(Pair.EmptyList, args);
        }

        private object _Append(object ls, object args) {
            return (args == Pair.EmptyList) ? ls : _BuildList(ls, args);
        }

        private object _BuildList(object ls, object args) {
            return (ls == Pair.EmptyList) ?
                _Append(List.First(args), List.Rest(args)) :
                new Pair(List.First(ls), _BuildList(List.Rest(ls), args));
        }
    }

    public class VectorLength : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Vector vec = (Vector)List.First(args);
            return (long)vec.Length;
        }
    }

    public class NumberPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is long);
        }
    }

    public class NumericEqualsPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            long len = List.Length(args);

            if (len < 2) {
                throw new System.Exception("= expects at least 2 arguments");
            } else {
                long currentInt = (long)args.Head;
                Pair currentPair = (Pair)args.Tail;
                while (currentPair != Pair.EmptyList) {
                    long nextInt = (long)List.First(currentPair);
                    if (currentInt != nextInt) {
                        return Boolean.Create(false);
                    }
                    currentPair = (Pair)currentPair.Tail;
                }

                return Boolean.Create(true);
            }
        }
    }

    public class GetProp : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Symbol s = (Symbol)List.First(args);
            Symbol prop = (Symbol)List.Second(args);

            return (s.Properties.Contains(prop)) ?
                s.Properties[prop] : Boolean.Create(false);
        }
    }

    public class RemProp : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Symbol s = (Symbol)List.First(args);
            s.Properties.Remove(List.Second(args));
            return null;
        }
    }

    public class ValuesProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            long len = List.Length(args);

            return (len > 1) ? new ValueList(args) : 
                (len == 1) ? List.First(args) : null;
        }
    }

    public class VoidProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return null;
        }
    }

    public class EqPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) == List.Second(args));
        }
    }

    public class StringPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is string);
        }
    }

    public class StringEqualsPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            string s1 = (string)List.First(args);
            string s2 = (string)List.Second(args);
            return Boolean.Create(s1 == s2);
        }
    }

    public class VectorPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is Vector);
        }
    }

    public class BooleanPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object o = List.First(args);
            return Boolean.Create(o == Boolean.TRUE || o == Boolean.FALSE);
        }
    }

    public class EqvPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object o1 = List.First(args);
            object o2 = List.Second(args);
            return Boolean.Create(
                (o1 is long && o2 is long) ? (long)o1 == (long)o2 :
                    (o1 is Character && o2 is Character) ?
                        (Character)o1 == (Character)o2 : o1 == o2
            );
        }
    }

    public class NullPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) == Pair.EmptyList);
        }
    }

    public class SymbolPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is Symbol);
        }
    }

    public class PairPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object o = List.First(args);
            return Boolean.Create(o != Pair.EmptyList && o is Pair);
        }
    }


    public class ProcedurePred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is Procedure);
        }
    }

    public class SetCdr : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Pair p = List.First(args) as Pair;

            if (p == null) {
                throw new System.Exception("Error: attempt to set cdr of non-pair argument");
            }

            p.Tail = List.Second(args);

            return null;
        }
    }

    public class SetCar : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Pair p = List.First(args) as Pair;

            if (p == null) {
                throw new System.Exception("Error: attempt to set car of non-pair argument");
            }

            p.Head = List.Second(args);

            return null;
        }
    }

    public class StringLength : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            string s = List.First(args) as string;
            if (s == null) {
                throw new System.Exception("Error: cannot apply string-length to non-string value");
            } else {
                return s.Length;
            }
        }
    }

    public class VectorCtor : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return List.ToVector(args);
        }
    }

    public class VectorRef : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object vec = List.First(args);
            long index = (long)List.Second(args);

            Vector v = vec as Vector;
            if (v == null) {
                throw new System.Exception("Attempt to index non-vector value: " + vec);
            } else if (index < v.Length) {
                return v[index];
            } else {
                throw new System.Exception("Index " + index + " out of range [0, " +
                    (v.Length - 1) + "] in vector " + v);
            }
        }
    }

    public class Load : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            System.IO.StreamReader reader = 
                new System.IO.StreamReader(List.First(args).ToString());
            i.Read(reader, env);
            return null;
        }
    }

    public class ZeroPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            int n = (int)List.First(args);
            return Boolean.Create(n == 0);
        }
    }

    public class Cons : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return new Pair(List.First(args), List.Second(args));
        }
    }

    public class Car : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object o = List.First(args);
            if (o == Pair.EmptyList) {
                throw new System.Exception("Cannot take car of empty list");
            } else {
                Pair p = o as Pair;
                if (p == null) {
                    throw new System.Exception("Error: cannot take car of non-pair");
                } else {
                    return p.Head;
                }
            }
        }
    }

    public class Cdr : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object o = List.First(args);
            if (o == Pair.EmptyList) {
                throw new System.Exception("Cannot take cdr of empty list");
            } else {
                Pair p = o as Pair;
                if (p == null) {
                    throw new System.Exception("Error: cannot take cdr of non-pair");
                } else {
                    return p.Tail;
                }
            }
        }
    }

    public class Display : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            System.Console.Write(List.First(args));
            return null;
        }
    }

    public class IntegerToChar : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return (char)((int)List.First(args));
        }
    }

    public class Add : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object n1 = List.First(args);
            object n2 = List.Second(args);
            if (n1 is int && n2 is int) {
                int i1 = (int)n1;
                int i2 = (int)n2;
                return i1 + i2;
            } else if (n1 is long && n2 is long) {
                long l1 = (long)n1;
                long l2 = (long)n2;
                return l1 + l2;
            }

            throw new System.Exception("Unrecognized addition");
        }
    }

    public class Subtract : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object n1 = List.First(args);
            object n2 = List.Second(args);
            if (n1 is int && n2 is int) {
                int i1 = (int)n1;
                int i2 = (int)n2;
                return i1 - i2;
            } else if (n1 is long && n2 is long) {
                long l1 = (long)n1;
                long l2 = (long)n2;
                return l1 - l2;
            }

            throw new System.Exception("Unrecognized subtraction");
        }
    }

    public class GreaterThan : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object n1 = List.First(args);
            object n2 = List.Second(args);
            if (n1 is long && n2 is long) {
                long i1 = (long)n1;
                long i2 = (long)n2;
                return Boolean.Create(i1 > i2);
            }

            throw new System.Exception("Unrecognized >");
        }
    }

    public class LessThan : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object n1 = List.First(args);
            object n2 = List.Second(args);
            if (n1 is long && n2 is long) {
                long i1 = (long)n1;
                long i2 = (long)n2;
                return Boolean.Create(i1<i2);
            }

            throw new System.Exception("Unrecognized <");
        }
    }

    public class ApplyProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Procedure p = (Procedure)List.First(args);
            return p.Apply(i, List.Flatten((Pair)List.Rest(args)), env);
        }
    }

    public class StringProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            System.Text.StringBuilder strBuffer = 
                new System.Text.StringBuilder();

            Pair currentPair = args;

            while (currentPair != Pair.EmptyList) {
                strBuffer.Append(((Character)List.First(currentPair)).NativeChar);
                currentPair = (Pair)currentPair.Tail;
                //Character c = List.First(currentPair) as Character;
                //if (c != null) {
                //    strBuffer.Append(c.NativeChar);
                //    currentPair = (Pair)currentPair.Tail;
                //} else {
                //    throw new System.Exception("Error: non-character value" +
                //        " passed to string function: " + 
                //        List.First(currentPair));
                //}
            }

            return strBuffer.ToString();
        }
    }

    public class ListProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return args;
        }
    }

    public class NativeEval : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return i.Execute(List.First(args), Pair.EmptyList, env);
        }
    }

    public class GenSymProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return SymbolFactory.Create();
        }
    }

    public class GenSymPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Symbol s = (Symbol)List.First(args);
            return Boolean.Create(s.IsGenSym());
        }
    }

    public class ListToVector : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return List.ToVector((Pair)List.First(args));
        }
    }

    public class MakeVector : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            long n = (long)List.First(args);
            Vector v = new Vector(n);

            if (List.Length(args) == 2) {
                object o = List.Second(args);
                for (long index = 0; index < n; ++index) {
                    v[index] = o;
                }
            }

            return v;
        }
    }

    public class VectorSet : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            Vector v = (Vector)List.First(args);
            long n = (long)List.Second(args);
            object val = List.Third(args);

            v[n] = val;

            return null;
        }
    }

    public class GetTicksProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return System.DateTime.UtcNow.Ticks;
        }
    }

    public class DivProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object n1 = List.First(args);
            object n2 = List.Second(args);
            if (n1 is long && n2 is long) {
                long i1 = (long)n1;
                long i2 = (long)n2;
                return i1 / i2;
            }

            throw new System.Exception("Unrecognized division");
        }
    }

    public class CallCC : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            throw new CaptureException((Procedure)List.First(args));
        }
/*
            ContinuationException kEx = new ContinuationException();
            Continuation k = new Continuation(kEx);

            try {
                Procedure p = (Procedure)List.First(args);
                return p.Apply(i, new Pair(k, Pair.EmptyList), env);
            } catch (ContinuationException e) {
                if (e == kEx) {
                    return e.ReturnValue;
                } else {
                    throw e;
                }
            }
        }
*/
    }
    
    public class CompileProc : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            object expr = List.First(args);
            Compiler c = new Compiler();
            return c.Compile(expr);
        }
    }

    public class CharPred : Procedure {
        public object Apply(Interpreter i, Pair args, Environment env) {
            return Boolean.Create(List.First(args) is Character);
        }
    }
}
