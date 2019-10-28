//
// Copyright 2006 Anthony J. Lorelli
// $Id: Compiler.cs 310 2006-03-26 22:50:47Z  $
//
namespace Scheme {
    public class Compiler {
        private MacroExpander _macroExp;

        public Compiler(MacroExpander macroExp) {
            _macroExp = macroExp;
        }

        public Compiler() { _macroExp = null; }

        public object Compile() {
            object o = _macroExp.Expand();
            return (o == null) ? o : Compile(o);
        }

        public object Compile(object expr) {
            if (expr is Pair) {
                Pair p = (Pair)expr;
                object head = p.Head;
                object tail = p.Tail;

                if (head == SymbolCache.IF) {
                    object alternative = (List.Length(tail) == 3) ?
                        Compile(List.Third(tail)) :
                        List.Create(Instruction.Constant, null);
                    return List.Create(Instruction.Conditional,
                        Compile(List.First(tail)), Compile(List.Second(tail)),
                        alternative);
                } else if (head == SymbolCache.LAMBDA) {
                    Pair bodyList = (Pair)List.Rest(tail);
                    object bodyExpr = CompileSequence(bodyList);
                    return List.Create(Instruction.Closure, 
                        List.First(tail), bodyExpr);
                } else if (head == SymbolCache.SET) {
                    return List.Create(Instruction.Assign,
                        List.First(tail), Compile(List.Second(tail)));
                } else if (head == SymbolCache.DEFINE) {
                    return List.Create(Instruction.Define,
                        List.First(tail), Compile(List.Second(tail)));
                } else if (head == SymbolCache.QUOTE) {
                    return List.Create(Instruction.Constant, List.First(tail));
                } else if (head == SymbolCache.BEGIN) {
                    return (tail == Pair.EmptyList) ? 
                        List.Create(Instruction.Constant, null) :
                        CompileSequence(List.Reverse((Pair)tail));
                } else if (head == SymbolCache.LETREC) {
                    object letrecExp = List.Create(Instruction.Apply,
                        List.Create(Instruction.Closure, Pair.EmptyList,
                            CompileLetrec(tail)), Pair.EmptyList);
                    return letrecExp;
                } else if (head == SymbolCache.CALLWITHVALUES) {
                    object producerExpr = List.Create(Instruction.Apply, 
                        Compile(List.First(tail)), Pair.EmptyList);
                    object consumerExpr = List.Create(Instruction.Apply,
                        Compile(List.Second(tail)), 
                        List.Create(Instruction.ArgMultiple,
                            producerExpr, Pair.EmptyList));
                    return consumerExpr;
                } else {
                    object applicationExpr = CompileApplication(head, tail);
                    return applicationExpr;
                }
            } else {
                return (expr is Symbol) ?
                    List.Create(Instruction.Variable, expr) :
                    List.Create(Instruction.Constant, expr);
            }
        }

        private object CompileLetrec(object expr) {
            object letrecExpr = ArrangeDefinitions(List.First(expr));
            if (letrecExpr != Pair.EmptyList) {
                List.Last(letrecExpr).Tail = List.Rest(expr);
            } else {
                letrecExpr = List.Rest(expr);
            }
            return CompileSequence(List.Reverse((Pair)letrecExpr));
        }

        private object ArrangeDefinitions(object defExprs) {
            if (defExprs == Pair.EmptyList) {
                return defExprs;
            } else {
                object def = List.First(defExprs);
                return (def == Pair.EmptyList) ? Pair.EmptyList :
                    new Pair(List.Create(SymbolCache.DEFINE, List.First(def),
                        List.Second(def)), 
                        ArrangeDefinitions(List.Rest(defExprs)));
            }
        }

        private object CompileSequence(Pair exprList) {
            if (exprList == Pair.EmptyList) {
                return exprList;
            } else {
                object expr = List.First(exprList);
                object rest = List.Rest(exprList);

                return List.Create(Instruction.Sequence, Compile(expr),
                    CompileSequence((Pair)rest));
            }
        }

        private object CompileApplication(object rator, object rands) {
            return List.Create(Instruction.Apply, Compile(rator),
                CompileArguments(rands));
        }

        private object CompileArguments(object rands) {
            return (rands == Pair.EmptyList) ? rands :
                List.Create(Instruction.Argument, Compile(List.First(rands)),
                    CompileArguments(List.Rest(rands)));
        }
    }
}
