use std::io::Cursor;
use std::path::PathBuf;

use scheme::parser::Parser;
use scheme::scanner::Scanner;
use scheme::{Interpreter, InterpreterShell, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn scanner_and_parser_literals_lists_and_vectors() {
    let interp = Interpreter::new_bare();
    let src = "(#t #f #\\space #\\newline #\\A -42 +7 (1 . 2) 'x `(a ,b ,@c) #(10 20))";
    let mut parser = Parser::new(Scanner::new(Cursor::new(src.as_bytes())), interp.symbols().clone());
    let val = parser.read().expect("valid parse").expect("non-eof");
    assert_eq!(
        val.to_string(),
        "(#t #f #\\space #\\newline #\\A -42 7 (1 . 2) (quote x) (quasiquote (a (unquote b) (unquote-splicing c))) #2(10 20))"
    );
    assert!(parser.read().expect("eof check").is_none());
}

#[test]
fn core_primitives_and_data_structures() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    assert_eq!(interp.eval_str("(+ 19 23)").unwrap().to_string(), "42");
    assert_eq!(interp.eval_str("(- 5)").unwrap().to_string(), "-5");
    assert_eq!(interp.eval_str("(* 6 7)").unwrap().to_string(), "42");
    assert_eq!(interp.eval_str("(zero? 0)").unwrap().to_string(), "#t");
    assert_eq!(interp.eval_str("(integer->char 65)").unwrap().to_string(), "#\\A");
    assert_eq!(interp.eval_str("(string-length \"scheme\")").unwrap().to_string(), "6");
    assert_eq!(
        interp
            .eval_str("(let ((v (vector 1 2 3))) (vector-set! v 1 99) v)")
            .unwrap()
            .to_string(),
        "#3(1 99 3)"
    );
}

#[test]
fn macros_hygiene_and_define_macro() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    let res = interp
        .eval_str(
            r#"
            (define-syntax my-or
              (syntax-rules ()
                ((my-or) #f)
                ((my-or e) e)
                ((my-or e1 e2 ...)
                 (let ((temp e1))
                   (if temp temp (my-or e2 ...))))))
            (let ((temp 42))
              (my-or #f temp))
            "#,
        )
        .unwrap();
    assert_eq!(res.to_string(), "42");

    let macro_res = interp
        .eval_str(
            r#"
            (define-macro (when-positive n . body)
              `(if (> ,n 0) (begin ,@body) 'non-positive))
            (list (when-positive 5 'ok1 'ok2) (when-positive -1 'ok1))
            "#,
        )
        .unwrap();
    assert_eq!(macro_res.to_string(), "(ok2 non-positive)");
}

#[test]
fn multiple_values_and_continuations() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");

    // Multiple values + continuation passed multiple values
    assert_eq!(
        interp
            .eval_str("(call-with-values (lambda () (values 10 20 12)) +)")
            .unwrap()
            .to_string(),
        "42"
    );
    assert_eq!(
        interp
            .eval_str("(call-with-values (lambda () (call/cc (lambda (k) (k 15 27)))) +)")
            .unwrap()
            .to_string(),
        "42"
    );

    // Multi-shot continuation re-invocation
    let multishot = interp
        .eval_str(
            r#"
            (let ((saved #f) (acc '()))
              (let ((v (+ 100 (call/cc (lambda (k) (set! saved k) 1)))))
                (set! acc (cons v acc))
                (if (< v 104)
                    (saved (- v 99))
                    (reverse acc))))
            "#,
        )
        .unwrap();
    assert_eq!(multishot.to_string(), "(101 102 103 104)");

    // Nested capture inside both operand and operator positions
    let both_positions = interp
        .eval_str(
            r#"
            ((call/cc (lambda (k1) (k1 (lambda (x) (+ x 40)))))
             (call/cc (lambda (k2) (k2 2))))
            "#,
        )
        .unwrap();
    assert_eq!(both_positions.to_string(), "42");
}

#[test]
fn proper_tail_recursion_handles_deep_loops() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    let res = interp
        .eval_str(
            r#"
            (let loop ((n 0) (acc 0))
              (if (= n 25000)
                  acc
                  (loop (+ n 1) (+ acc 1))))
            "#,
        )
        .unwrap();
    assert_eq!(res.to_string(), "25000");
}

#[test]
fn repl_handles_expressions_and_exits_on_eof() {
    let interp = Interpreter::with_captured_output().expect("init runtime");
    let mut shell = InterpreterShell::with_interpreter(interp);
    let input = Cursor::new(b"(define x 21)\n(* x 2)\n");
    let mut out = Vec::new();
    shell.repl(input, &mut out);
    let rendered = String::from_utf8(out).unwrap();
    assert_eq!(rendered, "> > 42\n> \n");
}

#[test]
fn legacy_test_scm_matches_expected_output() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    let test_path = repo_root().join("test").join("test.scm");
    let val = interp.load_file(&test_path).expect("test.scm should succeed");
    assert!(matches!(val, Value::Void));

    let out = interp.take_captured_output();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "3");
    assert_eq!(lines[1], "1");
    assert_eq!(lines[2], "1");
    assert!(lines[3].starts_with("(define make-fibo-coroutine"));
}

#[test]
fn shared_suite_scm_passes_all_assertions() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    let suite_path = repo_root().join("test").join("suite.scm");
    interp
        .load_file(&suite_path)
        .expect("suite.scm should execute cleanly");

    let out = interp.take_captured_output();
    assert!(!out.contains("FAIL ["), "Unexpected failures:\n{out}");
    assert!(out.contains("Failed: 0"), "Missing zero-failure summary:\n{out}");
}

#[test]
fn callcc_benchmarks_produce_exact_results() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");
    let test_dir = repo_root().join("test");
    interp.load_file(test_dir.join("coroutines.scm")).unwrap();
    interp.load_file(test_dir.join("exceptions.scm")).unwrap();
    interp.load_file(test_dir.join("benchmark.scm")).unwrap();

    assert_eq!(
        interp.eval_str("(test-driver)").unwrap().to_string(),
        "Divide-by-zero error"
    );
    assert_eq!(
        interp.eval_str("(get-fibo-list 2000)").unwrap().to_string(),
        "(1 2 3 5 8 13 21 34 55 89 144 233 377 610 987 1597)"
    );
    assert_eq!(interp.eval_str("(tak 18 12 6)").unwrap().to_string(), "7");
    assert_eq!(interp.eval_str("(ctak 18 12 6)").unwrap().to_string(), "7");
    assert_eq!(interp.eval_str("(capture-k 1000)").unwrap().to_string(), "done");
    assert_eq!(interp.eval_str("(invoke-k 1000)").unwrap().to_string(), "done");
}
