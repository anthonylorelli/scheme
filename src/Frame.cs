//
// Copyright 2006 Anthony J. Lorelli
// $Id: Frame.cs 298 2006-03-20 20:23:27Z  $
//
namespace Scheme {
    public class Frame {
        public object Expr;
        public Environment Env;
        public object Value;
        public Pair ValueRib;
        public Frame Context;
        private bool _captured;

        public Frame() {
            Expr = InstructionCache.End;
            Env = null;
            Value = null;
            ValueRib = null;
            Context = null;
            _captured = false;
        }

        public Frame(object expr, Frame context) {
            Expr = expr;
            Env = context.Env;
            Value = context.Value;
            ValueRib = context.ValueRib;
            Context = context.Context;
            _captured = false;
        }

        public Frame(object val, object expr, Frame context) {
            Expr = expr;
            Env = context.Env;
            Value = val;
            ValueRib = context.ValueRib;
            Context = context.Context;
            _captured = false;
        }

        public Frame(Frame context) {
            Expr = InstructionCache.End;
            Env = context.Env;
            Value = null;
            ValueRib = null;
            Context = context;
            _captured = false;
        }

        public Frame(Environment env) {
            Expr = InstructionCache.End;
            Env = env;
            Value = null;
            ValueRib = null;
            Context = null;
            _captured = false;
        }

        public Frame Clone() {
            Frame f = new Frame();

            f.Expr = Expr;
            f.Env = Env;
            f.Value = Value;
            f.ValueRib = ValueRib;
            f.Context = Context;
            f._captured = _captured;

            return f;
        }

        public bool Captured { get { return _captured; } }

        public void Mark() {
            _captured = true;
            if (Context != null) {
                Context.Mark();
            }
        }

        public override string ToString() {
            string s = "Frame\n" +
                "Expr: " + ((Expr == null) ? "[null]" : Expr) + "\n" +
                "Env: " + ((Env == null) ? "[null]" : Env.ToString()) + "\n" +
                "Value: " + ((Value == null) ? "[null]" : Value) + "\n" +
                "ValueRib: " + ((ValueRib == null) ? "[null]" : ValueRib.ToString()) + "\n" +
                "Context: " + ((Context == null) ? "[null]" : Context.ToString()) + "\n" +
                "Captured: " + _captured.ToString();
            return s;
        }
    }

    public class FrameFactory {
        private static System.Collections.Queue _freeList;

        static FrameFactory() {
            _freeList = new System.Collections.Queue();
        }

        public static Frame Create(object val, object expr, Frame context) {
            Frame f;
            if (_freeList.Count > 0) {
                f = (Frame)_freeList.Dequeue();
                f.Expr = expr;
                f.Value = val;
                f.ValueRib = context.ValueRib;
                f.Env = context.Env;
                f.Context = context.Context;
            } else {
                f = new Frame(val, expr, context);
            }

            if (!context.Captured) {
                _freeList.Enqueue(context);
            }

            return f;
        }

        public static Frame Create(object expr, Frame context) {
            Frame f;
            if (_freeList.Count > 0) {
                f = (Frame)_freeList.Dequeue();
                f.Expr = expr;
                f.Value = context.Value;
                f.ValueRib = context.ValueRib;
                f.Env = context.Env;
                f.Context = context.Context;
            } else {
                f = new Frame(expr, context);
            }

            if (!context.Captured) {
                _freeList.Enqueue(context);
            }

            return f;
        }

        public static void Release(Frame f) {
            if (!f.Captured) {
                _freeList.Enqueue(f);
            }
        }
    }
}
