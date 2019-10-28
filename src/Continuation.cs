//
// Copyright 2006 Anthony J. Lorelli
// $Id: Continuation.cs 314 2006-03-29 10:50:15Z  $
//
namespace Scheme {
    public class Continuation : Procedure {
        private ContextList _cList;

        public Continuation(ContextList cList) {
            _cList = cList;
        }

        public object Apply(Interpreter i, Pair args, Environment env) {
            InvokeException invEx = 
                new InvokeException(List.First(args), _cList);
            throw invEx;
        }
    }

    public class InvokeException : System.Exception { 
        private object _val;
        private ContextList _cList;

        public InvokeException(object val, ContextList cList) { 
            _val = val; 
            _cList = cList;
        }

        public object ReturnValue { 
            get { return _val; } 
        }

        public ContextList Continuation {
            get { return _cList; }
        }
    }

    public class CaptureException : System.Exception {
        private ContextList _cList;
        private Procedure _proc;

        public CaptureException(Procedure proc) {
            _cList = null;
            _proc = proc;
        }

        public void AddContext(Context c) {
            ContextList cList = new ContextList(c, _cList);
            if (_cList != null) {
                _cList.Older = cList;
            }
            _cList = cList;
        }

        public void AppendContext(ContextList cShared) {
//System.Console.WriteLine("Appending context:\n" + cShared.ToString());
            if (cShared != null) {
                ContextList cList = cShared.CopyStack();

                cList.Newer = _cList;

                if (_cList != null) {
                    _cList.Older = cList;

                    _cList = cList;

                } else {
                    _cList = cList;
                }

                while (_cList.Older != null) {
                    _cList = _cList.Older;
                }
            }
        }

        public ContextList Continuation {
            get { return _cList; }
        }

        public Procedure Procedure {
            get { return _proc; }
        }
    }
}
