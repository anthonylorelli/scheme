//
// Copyright 2006 Anthony J. Lorelli
// $Id: Environment.cs 314 2006-03-29 10:50:15Z  $
//
namespace Scheme {
    public class Environment {
        private System.Collections.Hashtable _symbolTable;
        private Environment _parentEnv;

        public Environment(Pair syms, Pair vals, Environment parentEnv) { 
            _symbolTable = new System.Collections.Hashtable();
            _parentEnv = parentEnv;
//System.Console.WriteLine("Syms: " + syms.ToString());
//System.Console.WriteLine("Vals: " + vals.ToString());
            CollectValues(syms, vals); 
        }

        public Environment(Pair syms, Pair vals) {
            _symbolTable = new System.Collections.Hashtable();
            _parentEnv = null;
//System.Console.WriteLine("Syms: " + syms.ToString());
//System.Console.WriteLine("Vals: " + vals.ToString());
            CollectValues(syms, vals);
        }

        public Environment(Environment parentEnv) {
            _symbolTable = new System.Collections.Hashtable();
            _parentEnv = parentEnv;
        }

        public Environment() {
            _symbolTable = new System.Collections.Hashtable();
            _parentEnv = null;
        }

        private void CollectValues(Pair syms, Pair vals) {
            if (syms != Pair.EmptyList) {
                _symbolTable[syms.Head] = vals.Head;
                CollectValues((Pair)syms.Tail, (Pair)vals.Tail);
            }
        }

        public object Lookup(Symbol s) {
            if (_symbolTable.Contains(s)) {
                return _symbolTable[s];
            } else if (_parentEnv != null) {
                return _parentEnv.Lookup(s);
            } else {
                return null;
            }
        }

        public bool Assign(Symbol s, object v) {
            if (_symbolTable.Contains(s)) {
                _symbolTable[s] = v;
                return true;
            } else if (_parentEnv != null) {
                return _parentEnv.Assign(s, v);
            } else {
                return false;
            }
        }

        public void Define(Symbol s, object v) {
            _symbolTable[s] = v;
        }
    }
}
