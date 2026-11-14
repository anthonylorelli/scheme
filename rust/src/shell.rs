use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::interpreter::Interpreter;
use crate::parser::Parser;
use crate::scanner::Scanner;
use crate::value::Value;

pub struct InterpreterShell {
    interpreter: Interpreter,
}

impl InterpreterShell {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            interpreter: Interpreter::new()?,
        })
    }

    pub fn with_interpreter(interpreter: Interpreter) -> Self {
        Self { interpreter }
    }

    pub fn interpreter_mut(&mut self) -> &mut Interpreter {
        &mut self.interpreter
    }

    pub fn read_file(&mut self, file: impl AsRef<Path>, writer: &mut impl Write) {
        if let Err(err) = self.interpreter.load_file(file) {
            let _ = writeln!(writer, "{err}");
        }
    }

    pub fn repl(&mut self, input: impl BufRead, writer: &mut impl Write) {
        let mut parser = Parser::new(Scanner::new(input), self.interpreter.symbols().clone());
        loop {
            let _ = write!(writer, "> ");
            let _ = writer.flush();

            match self.interpreter.eval_next(&mut parser) {
                Ok(Some(val)) => {
                    if !matches!(val, Value::Void) {
                        let _ = writeln!(writer, "{val}");
                    }
                }
                Ok(None) => {
                    let _ = writeln!(writer);
                    break;
                }
                Err(err) => {
                    let _ = writeln!(writer, "{err}");
                }
            }
        }
    }

    pub fn run_cli(args: &[String]) -> i32 {
        let mut stdout = io::stdout();
        let mut shell = match Self::new() {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(io::stderr(), "Failed to initialize Scheme runtime: {e}");
                return 1;
            }
        };

        if args.len() == 2 && args[0] == "-f" {
            shell.read_file(&args[1], &mut stdout);
            let stdin = io::stdin();
            shell.repl(stdin.lock(), &mut stdout);
        } else if args.len() == 1 {
            if let Err(err) = shell.interpreter.load_file(&args[0]) {
                let _ = writeln!(io::stderr(), "{err}");
                return 1;
            }
        } else if args.is_empty() {
            let stdin = io::stdin();
            shell.repl(stdin.lock(), &mut stdout);
        } else {
            let _ = writeln!(io::stderr(), "Usage: scheme [-f <file> | <file>]");
            return 2;
        }
        0
    }
}
