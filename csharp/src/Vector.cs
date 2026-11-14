//
// Copyright 2006 Anthony J. Lorelli
// $Id: Vector.cs 280 2006-03-12 09:46:35Z  $
//
namespace Scheme {
    public class Vector {
        private object[] _val;

        public Vector(object[] val) { _val = val; }

        public Vector(long n) { _val = new object[n]; }

        public int Length {
            get {
                return _val.Length;
            }
        }

        public object this[long index] {
            get { return _val[index]; }
            set { _val[index] = value; }
        }

        public override string ToString() {
            string s = "#" + _val.Length + "(";
            for (int i = 0; i < _val.Length; ++i) {
                if (i != 0) s += " ";
                s += _val[i];
            }
            s += ")";

            return s;
        }
    }
}
