//
// Copyright 2006 Anthony J. Lorelli
// $Id: Procedure.cs 299 2006-03-20 22:16:45Z  $
//
namespace Scheme {
    public interface Procedure {
        object Apply(Interpreter i, Pair args, Environment env);
    }
}
