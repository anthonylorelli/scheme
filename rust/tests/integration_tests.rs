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

#[test]
fn lexical_addressing_edge_cases() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");

    // Multi-level lexical shadowing + mutation across depths + zero-arg thunks (`has_frame = false`)
    let nested = interp
        .eval_str(
            r#"
            (let ((x 1) (y 10))
              (let ((bump (lambda () (set! x (+ x 1)) (+ x y))))
                (let ((x 100))
                  (let* ((r1 (bump))
                         (r2 (bump)))
                    (list x r1 r2)))))
            "#,
        )
        .unwrap();
    assert_eq!(nested.to_string(), "(100 12 13)");

    // Zero-arg mutually recursive thunks inside letrec (tests lambda boundary + frameless thunks)
    let mutual_thunks = interp
        .eval_str(
            r#"
            (let ((n 5) (acc 0))
              (letrec ((ping (lambda ()
                               (if (= n 0) acc (begin (set! acc (+ acc n)) (set! n (- n 1)) (pong)))))
                       (pong (lambda ()
                               (if (= n 0) acc (begin (set! acc (+ acc n)) (set! n (- n 1)) (ping))))))
                (ping)))
            "#,
        )
        .unwrap();
    assert_eq!(mutual_thunks.to_string(), "15");

    // Internal defines inside unexpanded lambda via _native-eval (including shorthand define)
    let internal_def = interp
        .eval_str(
            r#"
            (_native-eval
              '((lambda (a)
                  (define (helper z) (+ z b))
                  (define b 30)
                  (helper a))
                12))
            "#,
        )
        .unwrap();
    assert_eq!(internal_def.to_string(), "42");

    // Continuation captured inside global set! within a single top-level begin expression
    let global_cc = interp
        .eval_str(
            r#"
            (define *g-saved* #f)
            (define *g-log* '())
            (define *g-val* 0)
            (begin
              (set! *g-val* (call/cc (lambda (k) (set! *g-saved* k) 1)))
              (set! *g-log* (cons *g-val* *g-log*))
              (if (< *g-val* 3)
                  (*g-saved* (+ *g-val* 1))
                  (reverse *g-log*)))
            "#,
        )
        .unwrap();
    assert_eq!(global_cc.to_string(), "(1 2 3)");

    // Variadic lambda with rest list + local mutation
    let variadic = interp
        .eval_str(
            r#"
            ((lambda (a b . rest)
               (set! rest (cons (+ a b) rest))
               rest)
             3 4 10 20)
            "#,
        )
        .unwrap();
    assert_eq!(variadic.to_string(), "(7 10 20)");
}

#[test]
fn small_arity_and_spill_edge_cases() {
    let mut interp = Interpreter::with_captured_output().expect("init runtime");

    // 0-arg variadic inline, 2-arg variadic empty rest, 4-arg variadic inline, 6-arg heap spill
    assert_eq!(interp.eval_str("((lambda all all))").unwrap().to_string(), "()");
    assert_eq!(
        interp
            .eval_str("((lambda (a b . rest) (list a b rest)) 1 2)")
            .unwrap()
            .to_string(),
        "(1 2 ())"
    );
    assert_eq!(
        interp
            .eval_str("((lambda (a b . rest) (list a b rest)) 1 2 3 4)")
            .unwrap()
            .to_string(),
        "(1 2 (3 4))"
    );
    assert_eq!(
        interp
            .eval_str("((lambda (a b c d e f) (+ a b c d e f)) 1 2 3 4 5 6)")
            .unwrap()
            .to_string(),
        "21"
    );

    // Fixed <= 4 params whose internal defines spill total slots past SMALL_SLOTS (4)
    let spill_locals = interp
        .eval_str(
            r#"
            (_native-eval
              '((lambda (a b)
                  (define c (+ a 1))
                  (define d (+ b 2))
                  (define e (+ c d))
                  (+ a b c d e))
                10 20))
            "#,
        )
        .unwrap();
    assert_eq!(spill_locals.to_string(), "96");

    // Continuation capture mid-argument in both inline (4-arg) and heap (5-arg) calls
    let inline_capture = interp
        .eval_str(
            r#"
            (let ((saved #f) (log '()))
              (let ((res (list 1
                               (call/cc (lambda (k) (set! saved k) 2))
                               3
                               4)))
                (set! log (cons res log))
                (if (= (cadr res) 2)
                    (saved 20)
                    (reverse log))))
            "#,
        )
        .unwrap();
    assert_eq!(inline_capture.to_string(), "((1 2 3 4) (1 20 3 4))");

    let heap_capture = interp
        .eval_str(
            r#"
            (let ((saved #f) (log '()))
              (let ((res (list 1
                               2
                               (call/cc (lambda (k) (set! saved k) 3))
                               4
                               5)))
                (set! log (cons res log))
                (if (= (caddr res) 3)
                    (saved 30)
                    (reverse log))))
            "#,
        )
        .unwrap();
    assert_eq!(heap_capture.to_string(), "((1 2 3 4 5) (1 2 30 4 5))");

    // Continuation invoked with 5 multiple values (spilling past SMALL_SLOTS)
    let multi_val_k = interp
        .eval_str(
            r#"
            (call-with-values
              (lambda () (call/cc (lambda (k) (k 1 2 3 4 5))))
              (lambda (a b c d e) (+ a b c d e)))
            "#,
        )
        .unwrap();
    assert_eq!(multi_val_k.to_string(), "15");
}
