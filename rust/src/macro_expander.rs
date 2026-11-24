use crate::interpreter::Interpreter;
use crate::value::{Procedure, Value};

pub struct MacroExpander;

impl MacroExpander {
    pub fn expand(interpreter: &mut Interpreter, expr: Value) -> Result<Value, String> {
        let expander_sym = interpreter.symbols().borrow().cache.expander.clone();
        let maybe_expander = expander_sym.get_global();
        if let Some(Value::Procedure(proc)) = maybe_expander {
            interpreter.call_procedure_top_level(&proc, vec![expr])
        } else {
            Ok(expr)
        }
    }

    #[allow(dead_code)]
    pub fn has_custom_expander(interpreter: &Interpreter) -> bool {
        let expander_sym = interpreter.symbols().borrow().cache.expander.clone();
        matches!(
            expander_sym.get_global(),
            Some(Value::Procedure(Procedure::Closure(_)))
        )
    }
}
