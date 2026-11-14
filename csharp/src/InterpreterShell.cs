//
// Copyright 2006 Anthony J. Lorelli
// $Id: InterpreterShell.cs 300 2006-03-21 13:53:40Z  $
//
namespace Scheme {
    using System.IO;

    public class InterpreterShell {
        private System.IO.TextWriter _writer;
        private Interpreter _i;

        public InterpreterShell(System.IO.TextWriter writer, Interpreter i) {
            _writer = writer; _i = i;
        }

        public static void LoadRuntime(Interpreter interpreter, string baseDir) {
            interpreter.Read(Primitives.CreateReader());
            using (System.IO.StreamReader psyntax = new System.IO.StreamReader(Path.Combine(baseDir, "psyntax.pp"))) {
                interpreter.Read(psyntax);
            }
            using (System.IO.StreamReader init = new System.IO.StreamReader(Path.Combine(baseDir, "init.scm"))) {
                interpreter.Read(init);
            }
        }

        public static void Main(string[] args) {
            InterpreterShell shell = new InterpreterShell(
                System.Console.Out, new Interpreter(System.Console.In));

            string appDir = System.AppDomain.CurrentDomain.BaseDirectory;

            shell._i.Read(Primitives.CreateReader());
            shell.Read(Path.Combine(appDir, "psyntax.pp"));
            shell.Read(Path.Combine(appDir, "init.scm"));

            if (args.Length == 2 && args[0] == "-f") {
                shell.Read(args[1]);
                shell.REPL();
            } else if (args.Length == 1) {
                shell.Read(args[0]);
            } else if (args.Length == 0) {
                shell.REPL();
            }
        }

        public void Read(string file) {
            try {
                _i.ReadFile(file);
            } catch (System.Exception e) {
                _writer.WriteLine(e.Message);
                _writer.WriteLine(e.StackTrace);
            }
        }

        public void REPL() {
            while (!_i.IsEOF) {
                _writer.Write("> ");
                try {
                    object result = _i.Evaluate();
                    if (_i.IsEOF) {
                        _writer.WriteLine();
                        break;
                    }
                    if (result != null) _writer.WriteLine(result);
                } catch (System.Exception e) {
                    _writer.WriteLine(e.Message);
                    _writer.WriteLine(e.StackTrace);
                }
            }
        }
    }
}
