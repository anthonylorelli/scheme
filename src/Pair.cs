//
// Copyright 2006 Anthony J. Lorelli
// $Id: Pair.cs 240 2006-02-20 08:46:10Z  $
//
namespace Scheme {
    public class Pair {
        public static readonly Pair EmptyList;

        private object _head;
        private object _tail;

        static Pair() {
            EmptyList = new Pair(EmptyList, EmptyList);
        }

        public Pair(object head, object tail) { _head = head; _tail = tail; }

        public object Head {
            get { return _head; }
            set { _head = value; }
        }

        public object Tail {
            get { return _tail; }
            set { _tail = value; }
        }

        public override string ToString() {
            if (this == EmptyList) {
                return "()";
            } else {
                System.Text.StringBuilder buf = new System.Text.StringBuilder();
                buf.Append('(');
                buf.Append(_head.ToString());
                object rest = _tail;
                while (rest is Pair && rest != EmptyList) {
                    buf.Append(" ");
                    buf.Append(((Pair)rest).Head.ToString());
                    rest = ((Pair)rest).Tail;
                }
                if (rest != EmptyList) {
                    buf.Append(" . ");
                    buf.Append(rest.ToString());
                }
                buf.Append(')');

                return buf.ToString();
            }
        }
    }
}
