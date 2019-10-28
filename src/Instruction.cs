//
// Copyright 2006 Anthony J. Lorelli
// $Id: Instruction.cs 310 2006-03-26 22:50:47Z  $
//

/*
(apply expr args)
(argument first rest)
(argmultiple first rest)
(assign sym expr)
(closure args body)
(conditional expr texpr fexpr)
(constant expr)
(define sym expr)
(sequence first rest)
(variable expr)
*/
namespace Scheme {
    public class Instruction {
        public static readonly Instruction Apply;
        public static readonly Instruction ArgMultiple;
        public static readonly Instruction Argument;
        public static readonly Instruction Assign;
        public static readonly Instruction Closure;
        public static readonly Instruction Constant;
        public static readonly Instruction Context;
        public static readonly Instruction Define;
        public static readonly Instruction Sequence;
        public static readonly Instruction Quote;
        public static readonly Instruction Variable;
        public static readonly Instruction Conditional;
        public static readonly Instruction End;

        static Instruction() {
            Apply = new Instruction();
            ArgMultiple = new Instruction();
            Argument = new Instruction();
            Assign = new Instruction();
            Closure = new Instruction();
            Constant = new Instruction();
            Context = new Instruction();
            Define = new Instruction();
            Sequence = new Instruction();
            Quote = new Instruction();
            Variable = new Instruction();
            Conditional = new Instruction();
            End = new Instruction();
        }

        public override string ToString() {
            return (this == Apply ) ? "Apply" :
                (this == ArgMultiple) ? "ArgMultiple" :
                (this == Argument) ? "Argument" :
                (this == Assign) ? "Assign" :
                (this == Closure) ? "Closure" :
                (this == Constant) ? "Constant" :
                (this == Context) ? "Context" :
                (this == Define) ? "Define" :
                (this == Sequence) ? "Sequence" :
                (this == Quote) ? "Quote" :
                (this == Variable) ? "Variable" :
                (this == Conditional) ? "Conditional" :
                "End";
        }
    }

    public class InstructionCache {
        public static Pair Apply = new Pair(Instruction.Apply, Pair.EmptyList);
        public static Pair End = new Pair(Instruction.End, Pair.EmptyList);
    }
}
