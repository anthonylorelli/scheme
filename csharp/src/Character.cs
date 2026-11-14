//
// Copyright 2006 Anthony J. Lorelli
// $Id: Character.cs 256 2006-02-28 22:40:14Z  $
//
namespace Scheme {
    public class Character {
        private char _ch;

        public Character(char ch) { _ch = ch; }

        public static bool operator ==(Character x, Character y) {
            return (x == null || y == null) ? false : x._ch == y._ch;
        }

        public static bool operator !=(Character x, Character y) {
            return (x == null || y == null) ? true : x._ch != y._ch;;
        }

        public char NativeChar {
            get { return _ch; }
        }

        public override int GetHashCode() {
            return (int)_ch;
        }

        public override bool Equals(object o) {
            return (o is Character) ?
                this == (Character)o : (object)this == o;
        }

        public override string ToString() {
            switch (_ch) {
                case ' ':
                    return "#\\space";
                case '\n':
                    return "#\\newline";
                default:
                    return "#\\" + _ch;
            }
        }
    }
}
