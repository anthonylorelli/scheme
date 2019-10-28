//
// Copyright 2006 Anthony J. Lorelli
// $Id: Context.cs 315 2006-03-29 13:31:39Z  $
//
namespace Scheme {
    public abstract class Context {
        protected Interpreter _i;
        protected object _expr;
        protected Environment _env;

        public Context() {
            _i = null;
            _expr = null;
            _env = null;
        }

        public Context(Interpreter i, Environment env) {
            _i = i;
            _expr = null;
            _env = env;
        }

        public Context(Interpreter i, object expr, Environment env) {
            _i = i;
            _expr = expr;
            _env = env;
        }

        public abstract object Invoke(object val);
    }

    public class ContextList {
        private Context _first;
        private ContextList _newer;
        private ContextList _older;

        public ContextList(Context first) {
            _first = first; _newer = null; _older = null;
        }

        public ContextList(Context first, ContextList newer) {
            _first = first; _newer = newer; _older = null;
        }

        public ContextList(Context first, ContextList newer, ContextList older) {
            _first = first; _newer = newer; _older = older;
        }

        public ContextList CopyStack() {
            return Copy(null);
        }

        private ContextList Copy(ContextList newer) {
            ContextList newStack = new ContextList(_first, newer);
            newStack._older = (_older == null) ?
                _older : _older.Copy(newStack);

            return newStack;
        }

        public Context Current {
            get { return _first; }
        }

        public ContextList Newer {
            get { return _newer; }
            set { _newer = value; }
        }

        public ContextList Older {
            get { return _older; }
            set { _older = value; }
        }

        public override string ToString() {
            string s = _first.ToString();
            if (_newer != null) {
                s += "\n" + _newer.ToString();
            }
            return s;
        }
    }

    public class HaltContext : Context {
        public HaltContext() { }

        public override object Invoke(object val) {
            return val;
        }

        public override string ToString() {
            return "HaltContext";
        }
    }

    public class CondContext : Context {
        private object _texpr;
        private object _fexpr;

        public CondContext(Interpreter i, object texpr, 
            object fexpr, Environment env) : base(i, env) { 
            _texpr = texpr;
            _fexpr = fexpr;
        }

        public override object Invoke(object val) {
            return _i.Execute(Boolean.IsTrue(val) ?  
                _texpr : _fexpr, null, _env);
        }

        public override string ToString() {
            return "CondContext: [T] " + _texpr.ToString() + 
                " [F] " + _fexpr.ToString();
        }
    }

    public class AssignContext : Context {
        private Symbol _id;

        public AssignContext(Interpreter i, Symbol id, Environment env) :
            base(i, env) { 
            _id = id;
        }

        public override object Invoke(object val) {
            if (_env.Assign(_id, val)) {
                return null;
            } else {
                throw new System.Exception(
                    "Error: cannot set undefined identifier " + _id
                );
            }
        }

        public override string ToString() {
            return "AssignContext: " + _id.ToString();
        }
    }

    public class DefineContext : Context {
        private Symbol _id;

        public DefineContext(Interpreter i, Symbol id, Environment env) :
            base(i, env) { 
            _id = id;
        }

        public override object Invoke(object val) {
            _env.Define(_id, val);
            return null;
        }

        public override string ToString() {
            return "DefineContext: " + _id.ToString();
        }
    }

    public class ArgContext : Context {
        public ArgContext(Interpreter i, object expr, Environment env) :
            base(i, expr, env) { }

        public override object Invoke(object val) {
            object result;
            try {
                result = _i.Execute(_expr, null, _env);
            } catch (CaptureException capEx) {
                capEx.AddContext(new RibContext(val));
                throw;
            }
            return new Pair(result, val);
        }

        public override string ToString() {
            return "ArgContext: " + _expr.ToString();
        }
    }

    public class RibContext : Context {
        private object _valueRib;

        public RibContext(object valueRib) {
            _valueRib = valueRib;
        }

        public override object Invoke(object val) {
            return new Pair(val, _valueRib);
        }

        public override string ToString() {
            return "RibContext: " + _valueRib.ToString();
        }
    }

    public class FuncContext : Context {
        public FuncContext(Interpreter i, object expr, Environment env) :
            base(i, expr, env) { }

        public override object Invoke(object val) {
            Pair vRib = (Pair)val;
            object fn = _i.Execute(_expr, null, _env);
            Procedure proc = (Procedure)fn;
            return proc.Apply(_i, vRib, _env);
        }

        public override string ToString() {
            return "FuncContext: " + _expr.ToString();
        }
    }

    public class ApplyContext : Context {
        private Pair _valueRib;

        public ApplyContext(Interpreter i, Pair valueRib, Environment env) : 
            base(i, env) {
            _valueRib = valueRib;
        }

        public override object Invoke(object val) {
            Procedure proc = (Procedure)val;
            return proc.Apply(_i, _valueRib, _env);
        }

        public override string ToString() {
            return "ApplyContext: " + _valueRib.ToString();
        }
    }

    public class SeqContext : Context {
        public SeqContext(Interpreter i, object expr, Environment env) :
            base(i, expr, env) { }

        public override object Invoke(object val) {
            return _i.Execute(_expr, null, _env);
        }

        public override string ToString() {
            return "SeqContext: " + _expr.ToString();
        }
    }

    public class MultArgContext : Context {
        public MultArgContext() { }

        public override object Invoke(object multArgVal) {
            Pair newRib = (multArgVal is ValueList) ?
                ((ValueList)multArgVal).Values :
                (multArgVal != null) ? 
                    new Pair(multArgVal, Pair.EmptyList) :
                    Pair.EmptyList;
            return newRib;
        }

        public override string ToString() {
            return "MultArgContext";
        }
    }
}
