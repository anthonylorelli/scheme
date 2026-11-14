//
// Copyright 2006 Anthony J. Lorelli
// $Id: Boolean.cs 220 2006-02-09 21:12:28Z  $
//
namespace Scheme {
    public class Boolean {
        public static readonly Boolean TRUE = new Boolean();
        public static readonly Boolean FALSE = new Boolean();

        private Boolean() { }

        public static Boolean Create(string s) { 
            return ( s.ToLower() == "#t" ) ? TRUE : FALSE;
        }

        public static Boolean Create(bool b) {
            return ( b ) ? TRUE : FALSE;
        }

        public static bool IsTrue(object o) {
            return ( o == FALSE ) ? false : true;
        }

        public override string ToString() {
            return ( this == TRUE ) ? "#t" : "#f";
        }
    }
}
