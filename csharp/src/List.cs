//
// Copyright 2006 Anthony J. Lorelli
// $Id: List.cs 311 2006-03-27 22:55:15Z  $
//
namespace Scheme {
    public class List {
        public static object First(object o) {
            Pair p = o as Pair;
            return (p != null) ? p.Head : null;
        }

        public static object Second(object o) {
            Pair p = o as Pair;
            return (p != null) ? First(p.Tail) : null;
        }

        public static object Third(object o) {
            Pair p = o as Pair;
            return (p != null) ? Second(p.Tail) : null;
        }

        public static object Fourth(object o) {
            Pair p = o as Pair;
            return (p != null) ? Third(p.Tail) : null;
        }

        public static object Rest(object o) {
            Pair p = o as Pair;
            return (p != null) ? p.Tail : null;
        }

        public static Pair Last(object o) {
            Pair p = o as Pair;
            if (p != null) {
                return (p.Tail == Pair.EmptyList) ?
                    p : (p.Tail is Pair) ? Last(p.Tail) : p;
            } else {
                throw new System.Exception(
                    "Error: attempt to perform list operation on non-list."
                );
            }
        }

        public static long Length(object o) {
            if (o == Pair.EmptyList) return 0L;
            Pair p = o as Pair;
            if (p != null) {
                return 1L + Length(p.Tail);
            } else {
                throw new System.Exception(
                    "Error: attempt to take length of non-list."
                );
            }
        }

        public static Pair Reverse(Pair p) {
            return Reverse(p, Pair.EmptyList);
        }

        private static Pair Reverse(Pair orig, Pair revList) {
            return (orig == Pair.EmptyList) ? revList :
                Reverse((Pair)List.Rest(orig), 
                    new Pair(List.First(orig), revList));
        }

        public static Pair Flatten(Pair args) {
            return (args.Tail == Pair.EmptyList) ?
                (Pair)args.Head : 
                new Pair(args.Head, Flatten((Pair)args.Tail));
        }

        public static Pair Create(System.Array a) {
            return Create(a.GetEnumerator());
        }

        private static Pair Create(System.Collections.IEnumerator e) {
            return ( e.MoveNext() ) ? 
                new Pair(e.Current, Create(e)) : Pair.EmptyList;
        }

        public static Pair Create(object e1) {
            return new Pair(e1, Pair.EmptyList);
        }

        public static Pair Create(object e1, object e2) {
            return new Pair(e1, new Pair(e2, Pair.EmptyList));
        }

        public static Pair Create(object e1, object e2, object e3) {
            return new Pair(e1, new Pair(e2, new Pair(e3, Pair.EmptyList)));
        }

        public static Pair Create(object e1, object e2, object e3, object e4) {
            return new Pair(e1, new Pair(e2, 
                new Pair(e3, new Pair(e4, Pair.EmptyList))));
        }

        public static Vector ToVector(Pair p) {
            long l = Length(p);
            object[] vec = new object[l];
            Pair currentCons = p;

            for (int i = 0; i < l; ++i) {
                vec[i] = currentCons.Head;
                currentCons = (Pair)currentCons.Tail;
            }

            return new Vector(vec);
        }

        public static bool IsProper(Pair p) {
            Pair last = Last(p);
            return (last.Tail == Pair.EmptyList);
        }
    }
}
