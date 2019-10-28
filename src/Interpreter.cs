//
// Copyright 2006 Anthony J. Lorelli
// $Id: Interpreter.cs 315 2006-03-29 13:31:39Z  $
//
namespace Scheme {
    using System.IO;

    public class Interpreter {
        private Environment _globalEnv;
        private Compiler _compiler;
        private CaptureException _capEx;
        private InvokeException _invEx;

        public Interpreter(TextReader reader) {
            StandardLibrary sl = StandardLibrary.Instance();
            _globalEnv = new Environment(sl.Symbols, sl.Values);
            _compiler = new Compiler(new MacroExpander(new Parser(
                new Scanner(reader)), this));
            _capEx = null;
            _invEx = null;
        }

        public Environment GlobalEnvironment {
            get { return _globalEnv; }
        }

        public object Evaluate() {
            object o = _compiler.Compile();

            if (o == null) return o;

            return Evaluate(o, _globalEnv);
        }

        private object Evaluate(object expr, Environment env) {
            for (;;) {
                try {
                    return Dispatch(expr, env);
                } catch (CaptureException captureEx) {
                    _capEx = captureEx;
                } catch (InvokeException invEx) {
                    _invEx = invEx;
                }
            }
        }

        private object Dispatch(object expr, Environment env) {
            if (_capEx != null) {
                CaptureException capEx = _capEx;
                _capEx = null;
                if (capEx.Continuation == null) {
                    capEx.AddContext(new HaltContext());
                }
                Continuation k = new Continuation(capEx.Continuation);
                return ResumeWithProcedure(capEx.Continuation,
                    capEx.Procedure, k);
            } else if (_invEx != null) {
                InvokeException invEx = _invEx;
                _invEx = null;
                return ResumeWithValue(invEx.Continuation, invEx.ReturnValue);
            } else {
                return Execute(expr, Pair.EmptyList, env);
            }
        }

        private object ResumeWithProcedure(ContextList cList, 
            Procedure proc, Continuation k) {
            object returnValue;
            if (cList.Newer == null) {
                try {
                    returnValue = proc.Apply(this, List.Create(k), null);
                } catch (CaptureException capEx) {
                    capEx.AppendContext(cList);
                    throw;
                }
            } else {
                returnValue = ResumeWithProcedure(cList.Newer, proc, k);
            }

            try {
                return cList.Current.Invoke(returnValue);
            } catch (CaptureException capEx) {
//System.Console.WriteLine("Older: " + cList.Older.ToString());
                capEx.AppendContext(cList.Older);
                throw;
            }
        }

        private object ResumeWithValue(ContextList cList, object val) {
            object returnValue;
            if (cList.Newer == null) {
                returnValue = val;
            } else {
                returnValue = ResumeWithValue(cList.Newer, val);
            }

            try {
                return cList.Current.Invoke(returnValue);
            } catch (CaptureException capEx) {
//System.Console.WriteLine("Older: " + cList.Older.ToString());
                capEx.AppendContext(cList.Older);
                throw;
            }
        }

        public object Execute(object expr, Pair valueRib, Environment env) {
            object instr = List.First(expr);

            if (instr == Instruction.Constant) {
                return List.Second(expr);
            } else if (instr == Instruction.Variable) {
                object sym = List.Second(expr);
                object varValue = env.Lookup((Symbol)sym);
                if (varValue == null) {
                    throw new System.Exception(
                        "Error: no value associated with symbol " + sym
                    );
                } else {
                    return varValue;
                }
            } else if (instr == Instruction.Conditional) {                
                object cond = null;
                try {
                    cond = Execute(List.Second(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new CondContext(this, 
                        List.Third(expr), List.Fourth(expr), env));
                    throw capEx;
                }

                object condResult = Execute(Boolean.IsTrue(cond) ?
                    List.Third(expr) : List.Fourth(expr), 
                    valueRib, env);
                return condResult;
            } else if (instr == Instruction.Closure) {
                return new Closure((Pair)List.Second(expr),
                    (Pair)List.Third(expr), env);
            } else if (instr == Instruction.Assign) {
                object assignVal = null;
                Symbol id = (Symbol)List.Second(expr);
                try {
                    assignVal = Execute(List.Third(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new AssignContext(this, id, env));
                    throw;
                }
                if (env.Assign(id, assignVal)) {
                    return null;
                } else {
                    throw new System.Exception(
                        "Error: cannot set undefined identifier " + id
                    );
                }
            } else if (instr == Instruction.Define) {
                object defineVal = null;
                Symbol id = (Symbol)List.Second(expr);
                try {
                    defineVal = Execute(List.Third(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new DefineContext(this, id, env));
                    throw;
                }
                env.Define(id, defineVal);
                return null;
            } else if (instr == Instruction.Context) {
                return Execute(List.Second(expr), Pair.EmptyList, env);
            } else if (instr == Instruction.Argument) {
                Pair p = null;
                try {
                    object argRest = List.Third(expr);
                    p = (argRest == Pair.EmptyList) ? Pair.EmptyList :
                        (Pair)Execute(argRest, valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new ArgContext(this, 
                        List.Second(expr), env));
                    throw;
                }
                object argVal;
                try {
                    argVal = Execute(List.Second(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new RibContext(p));
                    throw;
                }
                return new Pair(argVal, p);
            } else if (instr == Instruction.Apply) {
                Pair vRib = null;
                try {
                    object fnArgs = List.Third(expr);
                    vRib = (fnArgs == Pair.EmptyList) ? Pair.EmptyList :
                        (Pair)Execute(fnArgs, valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new FuncContext(this, 
                        List.Second(expr), env));
                    throw;
                }
                object fn = null; 
                try {
                    fn = Execute(List.Second(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new ApplyContext(this, vRib, env));
                    throw;
                }
                Procedure proc = (Procedure)fn;
                return proc.Apply(this, vRib, env);
            } else if (instr == Instruction.ArgMultiple) {
                object multArgVal;
                try {
                    multArgVal = Execute(List.Second(expr), valueRib, env);
                } catch (CaptureException capEx) {
                    capEx.AddContext(new MultArgContext());
                    throw;
                }
                Pair newRib = (multArgVal is ValueList) ?
                    ((ValueList)multArgVal).Values :
                    (multArgVal != null) ? 
                        new Pair(multArgVal, Pair.EmptyList) :
                        Pair.EmptyList;
                return newRib;
            } else if (instr == Instruction.Sequence) {
                try {
                    object restSeq = List.Third(expr);
                    if (restSeq != Pair.EmptyList) {
                        Execute(restSeq, valueRib, env);
                    }
                } catch (CaptureException capEx) {
                    capEx.AddContext(new SeqContext(this, 
                        List.Second(expr), env));
                    throw;
                }
                return Execute(List.Second(expr), valueRib, env);
            } else {
                throw new System.Exception("Unexpected value: " + 
                    ((instr==null)?"[null]":instr));
            }
        }

        public void Read(TextReader reader) {
            Read(reader, _globalEnv);
        }

        public void Read(TextReader reader, Environment env) {
            Compiler compiler = new Compiler(new MacroExpander(
                new Parser(new Scanner(reader)), this));
            object o;
            while ((o = compiler.Compile()) != null) {
                try {
                    Evaluate(o, env);
                } catch (System.Exception e) { 
                    System.Console.WriteLine(e.Message + "\n" + e.StackTrace);
                }
            }
        }
    }
}
