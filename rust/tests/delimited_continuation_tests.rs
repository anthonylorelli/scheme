use std::cell::RefCell;

use scheme::Interpreter;

thread_local! {
    static SHARED_INTERP: RefCell<Interpreter> = RefCell::new(
        Interpreter::with_captured_output().expect("initialize Scheme runtime")
    );
}

fn eval(code: &str) -> String {
    SHARED_INTERP.with(|cell| {
        cell.borrow_mut()
            .eval_str(code)
            .unwrap_or_else(|e| panic!("Scheme evaluation failed: {e}\nCode:\n{code}"))
            .to_string()
    })
}

fn eval_err(code: &str) -> String {
    SHARED_INTERP.with(|cell| {
        cell.borrow_mut()
            .eval_str(code)
            .expect_err(&format!("Expected error for:\n{code}"))
    })
}

#[test]
fn prompt_tags_identity_and_predicates() {
    assert_eq!(
        eval("(continuation-prompt-tag? (default-continuation-prompt-tag))"),
        "#t"
    );
    assert_eq!(
        eval("(eq? (default-continuation-prompt-tag) (default-continuation-prompt-tag))"),
        "#t"
    );
    assert_eq!(
        eval("(let ((t1 (make-continuation-prompt-tag 'a)) (t2 (make-continuation-prompt-tag 'a))) (list (continuation-prompt-tag? t1) (continuation-prompt-tag? 42) (eq? t1 t1) (eq? t1 t2)))"),
        "(#t #f #t #f)"
    );
    assert_eq!(
        eval("(make-continuation-prompt-tag)"),
        "<#prompt-tag>"
    );
    assert_eq!(
        eval("(make-continuation-prompt-tag 'effect)"),
        "<#prompt-tag:effect>"
    );
    assert_eq!(
        eval("(make-continuation-prompt-tag \"custom\")"),
        "<#prompt-tag:custom>"
    );
}

#[test]
fn prompt_and_abort_default_and_custom_handlers() {
    // Normal return through a prompt
    assert_eq!(
        eval("(+ 1 (call-with-continuation-prompt (lambda () (+ 20 21))))"),
        "42"
    );

    // Default handler expects a thunk and re-installs the prompt around it (allowing nested aborts)
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'p)))
              (call-with-continuation-prompt
                (lambda ()
                  (+ 100
                     (abort-current-continuation tag
                       (lambda ()
                         (+ 200
                            (abort-current-continuation tag
                              (lambda () 42)))))))
                tag))
            "#
        ),
        "42"
    );

    // Custom handler receives multiple values and runs OUTSIDE the removed prompt
    assert_eq!(
        eval(
            r#"
            (let ((outer (make-continuation-prompt-tag 'outer))
                  (inner (make-continuation-prompt-tag 'inner)))
              (call-with-continuation-prompt
                (lambda ()
                  (call-with-continuation-prompt
                    (lambda ()
                      (+ 999 (abort-current-continuation inner 10 20 12)))
                    inner
                    (lambda (a b c) (+ a b c))))
                outer
                (lambda (x) 'wrong)))
            "#
        ),
        "42"
    );

    // Custom handler with 0 values
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'empty)))
              (call-with-continuation-prompt
                (lambda () (abort-current-continuation tag))
                tag
                (lambda () 'aborted-empty)))
            "#
        ),
        "aborted-empty"
    );

    // Missing custom prompt tag produces a clean runtime error
    let err = eval_err(
        r#"
        (let ((tag (make-continuation-prompt-tag 'missing)))
          (abort-current-continuation tag 1))
        "#,
    );
    assert!(
        err.contains("No continuation prompt found"),
        "Unexpected error: {err}"
    );

    let cap_err = eval_err(
        r#"
        (let ((tag (make-continuation-prompt-tag 'missing-cap)))
          (call-with-composable-continuation (lambda (k) (k 1)) tag))
        "#,
    );
    assert!(
        cap_err.contains("No continuation prompt found"),
        "Unexpected error: {cap_err}"
    );
}

#[test]
fn composable_continuation_semantics_and_multiple_values() {
    // Without aborting, the return value of `proc` flows into the current continuation
    assert_eq!(
        eval(
            r#"
            (+ 1 (reset
                   (+ 10 (call-with-composable-continuation
                           (lambda (k)
                             (if (procedure? k)
                                 (k (k 5))
                                 -1))))))
            "#
        ),
        // k = (+ 10 []), (k 5) = 15, (k 15) = 25, returned into (+ 10 25) = 35, + 1 = 36
        "36"
    );

    // Composable continuation invoked with multiple values and zero values
    assert_eq!(
        eval(
            r#"
            (reset
              (call-with-values
                (lambda () (shift k (+ 100 (k 10 20 12))))
                (lambda (a b c) (+ a b c))))
            "#
        ),
        "142"
    );
    assert_eq!(
        eval(
            r#"
            (reset
              (call-with-values
                (lambda () (shift k (list 'got (k))))
                (lambda () 'zero-ok)))
            "#
        ),
        "(got zero-ok)"
    );

    // Composable continuation escapes its prompt and composes inside a new context
    assert_eq!(
        eval(
            r#"
            (let ((saved #f))
              (let ((first (reset
                             (* 2 (call-with-composable-continuation
                                    (lambda (k) (set! saved k) 3))))))
                (list first (+ 100 (saved 5)) (+ 1000 (saved (saved 4))))))
            "#
        ),
        "(6 110 1016)"
    );
}

#[test]
fn danvy_filinski_shift_reset_patterns() {
    // Classic doubling & nested nondeterminism
    assert_eq!(
        eval("(+ 1 (reset (* 2 (shift k (k (k 10))))))"),
        "41"
    );
    assert_eq!(
        eval(
            r#"
            (reset
              (let* ((x (shift k (append (k 1) (k 2) (k 3))))
                     (y (shift k (append (k 10) (k 20)))))
                (list (+ x y))))
            "#
        ),
        "(11 21 12 22 13 23)"
    );

    // Collecting emitted values without mutable state
    assert_eq!(
        eval(
            r#"
            (let ((emit (lambda (v) (shift k (cons v (k 'ignored))))))
              (reset
                (emit 'a)
                (emit 'b)
                (emit 'c)
                '()))
            "#
        ),
        "(a b c)"
    );

    // Pure functional same-fringe using shift/reset (zero `set!`)
    assert_eq!(
        eval(
            r#"
            (letrec ((tree->stream
                      (lambda (tree)
                        (reset
                          (letrec ((walk
                                    (lambda (n)
                                      (cond
                                        ((null? n) '())
                                        ((pair? n) (walk (car n)) (walk (cdr n)))
                                        (else (shift k (cons n (lambda () (k 'ignored)))))))))
                            (walk tree)
                            '()))))
                     (same-fringe?
                      (lambda (t1 t2)
                        (let loop ((s1 (tree->stream t1)) (s2 (tree->stream t2)))
                          (cond
                            ((and (null? s1) (null? s2)) #t)
                            ((or (null? s1) (null? s2)) #f)
                            ((equal? (car s1) (car s2))
                             (loop ((cdr s1)) ((cdr s2))))
                            (else #f))))))
              (list (same-fringe? '(((1 . 2) . 3) . (4 . 5))
                                  '(1 . (2 . (3 . (4 . 5)))))
                    (same-fringe? '((1 . 2) . (99 . 4))
                                  '((1 . 2) . (3 . 4)))))
            "#
        ),
        "(#t #f)"
    );
}

#[test]
fn control_prompt_vs_shift_reset_distinction() {
    // Classic static (shift/reset) vs dynamic (control/prompt) distinction:
    // - Under shift/reset, invoking `k1` re-installs a reset around `k1`, so `k2` only captures
    //   up to `k1`'s boundary, returning `(b)` to `(cons 'a [])` => `(a b)`.
    // - Under control/prompt, invoking `k1` does NOT install a prompt, so `k2` captures and
    //   discards `(cons 'a [])` all the way to the outer prompt => `(b)`.
    assert_eq!(
        eval(
            r#"
            (list
              (reset
                (let ((x (shift k1 (cons 'a (k1 '())))))
                  (shift k2 (cons 'b x))))
              (prompt
                (let ((x (control k1 (cons 'a (k1 '())))))
                  (control k2 (cons 'b x)))))
            "#
        ),
        "((a b) (b))"
    );
}

#[test]
fn multi_prompt_isolation_and_preservation_across_captures() {
    // Capturing up to outer tag `t1` across inner tag `t2` preserves `t2` when `k` is replayed
    assert_eq!(
        eval(
            r#"
            (let ((t1 (make-continuation-prompt-tag 't1))
                  (t2 (make-continuation-prompt-tag 't2)))
              (reset-at t1
                (+ 1
                   (call-with-continuation-prompt
                     (lambda ()
                       (let ((x (shift-at t1 k (list (k 10) (k 'abort-inner)))))
                         (if (eq? x 'abort-inner)
                             (abort-current-continuation t2 99)
                             (+ x 100))))
                     t2
                     (lambda (v) (+ v 1000))))))
            "#
        ),
        // (k 10) => (+ 1 (+ 10 100)) = 111
        // (k 'abort-inner) => t2 catches 99 => (+ 99 1000) = 1099 => (+ 1 1099) = 1100
        "(111 1100)"
    );

    // Aborting to `t2` directly from inside `call-with-composable-continuation(..., t1)`
    // must still be caught by the enclosing inner prompt `t2`!
    assert_eq!(
        eval(
            r#"
            (let ((t1 (make-continuation-prompt-tag 't1))
                  (t2 (make-continuation-prompt-tag 't2)))
              (call-with-continuation-prompt
                (lambda ()
                  (call-with-continuation-prompt
                    (lambda ()
                      (call-with-composable-continuation
                        (lambda (k) (abort-current-continuation t2 'caught-inner))
                        t1))
                    t2
                    (lambda (v) (list 't2 v))))
                t1
                (lambda (v) (list 't1 v))))
            "#
        ),
        "(t2 caught-inner)"
    );

    // 3 nested prompts (t1 -> t2 -> t3) captured by t1, then replayed aborting to t3, t2, and t1
    assert_eq!(
        eval(
            r#"
            (let ((t1 (make-continuation-prompt-tag 't1))
                  (t2 (make-continuation-prompt-tag 't2))
                  (t3 (make-continuation-prompt-tag 't3)))
              (call-with-continuation-prompt
                (lambda ()
                  (let ((captured #f))
                    (let ((res
                           (call-with-continuation-prompt
                             (lambda ()
                               (+ 10
                                  (call-with-continuation-prompt
                                    (lambda ()
                                      (+ 100
                                         (call-with-continuation-prompt
                                           (lambda ()
                                             (let ((cmd (call-with-composable-continuation
                                                          (lambda (k)
                                                            (if (not captured) (set! captured k))
                                                            'normal)
                                                          t1)))
                                               (cond
                                                 ((eq? cmd 'normal) 1)
                                                 ((eq? cmd 'abort3) (abort-current-continuation t3 3))
                                                 ((eq? cmd 'abort2) (abort-current-continuation t2 2)))))
                                           t3
                                           (lambda (v) (+ v 300)))))
                                    t2
                                    (lambda (v) (+ v 2000)))))
                             t1
                             (lambda (v) v))))
                      (list res
                            (captured 'abort3)
                            (captured 'abort2)))))
                (make-continuation-prompt-tag 'guard)))
            "#
        ),
        // normal: 10 + (100 + 1) = 111
        // abort3: t3 handler -> 303, +100 -> 403, +10 -> 413
        // abort2: t2 handler -> 2002, +10 -> 2012
        "(111 413 2012)"
    );
}

#[test]
fn interaction_with_undelimited_call_cc() {
    // Capturing undelimited call/cc inside a prompt preserves the prompt on re-entry
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'p))
                  (saved #f)
                  (log '()))
              (let ((res
                     (+ 1
                        (call-with-continuation-prompt
                          (lambda ()
                            (let ((v (call/cc (lambda (k) (set! saved k) 'first))))
                              (if (eq? v 'second)
                                  (abort-current-continuation tag 50)
                                  10)))
                          tag
                          (lambda (x) (+ x 100))))))
                (set! log (cons res log))
                (if (= res 11)
                    (saved 'second)
                    (reverse log))))
            "#
        ),
        // first: (+ 1 10) = 11 -> invokes (saved 'second) -> aborts to reconstructed prompt tag
        // -> handler (+ 50 100) = 150 -> (+ 1 150) = 151
        "(11 151)"
    );

    // Capturing a delimited continuation inside a resumed raw composable continuation
    assert_eq!(
        eval(
            r#"
            (let ((raw-k #f))
              (reset
                (+ 1
                   (let ((x (call-with-composable-continuation (lambda (k) (set! raw-k k) 0))))
                     (if (= x 0)
                         0
                         (+ x (shift k2 (list 'captured (k2 10))))))))
              (reset (+ 1000 (raw-k 5))))
            "#
        ),
        // k2 = (+ 1000 (+ 1 (+ 5 []))), (k2 10) = 1016
        "(captured 1016)"
    );
}

#[test]
fn algebraic_effect_handlers_state_and_nondeterminism() {
    // Pure functional stacked algebraic effect handlers:
    // - State effect (`get`, `put`) on `state-tag` (zero `set!`)
    // - Nondeterminism (`choose`, `fail`) on `amb-tag` (zero `set!`)
    assert_eq!(
        eval(
            r#"
            (let ((state-tag (make-continuation-prompt-tag 'state))
                  (amb-tag (make-continuation-prompt-tag 'amb)))
              (let ((get (lambda ()
                           (call-with-composable-continuation
                             (lambda (k)
                               (abort-current-continuation state-tag 'get k))
                             state-tag)))
                    (put (lambda (new-s)
                           (call-with-composable-continuation
                             (lambda (k)
                               (abort-current-continuation state-tag 'put new-s k))
                             state-tag)))
                    (choose (lambda (xs)
                              (call-with-composable-continuation
                                (lambda (k)
                                  (abort-current-continuation amb-tag xs k))
                                amb-tag))))
                (letrec ((handle-state
                          (lambda (body-thunk)
                            (call-with-continuation-prompt
                              body-thunk
                              state-tag
                              (lambda (op . args)
                                (cond
                                  ((eq? op 'get)
                                   (let ((k (car args)))
                                     (lambda (s)
                                       ((handle-state (lambda () (k s))) s))))
                                  ((eq? op 'put)
                                   (let ((new-s (car args))
                                         (k (cadr args)))
                                     (lambda (s)
                                       ((handle-state (lambda () (k 'ok))) new-s)))))))))
                         (run-state
                          (lambda (init thunk)
                            ((handle-state
                               (lambda ()
                                 (let ((res (thunk)))
                                   (lambda (s) (cons res s)))))
                             init)))
                         (handle-amb
                          (lambda (body-thunk)
                            (call-with-continuation-prompt
                              body-thunk
                              amb-tag
                              (lambda (xs k)
                                (let loop ((ys xs))
                                  (if (null? ys)
                                      '()
                                      (append (handle-amb (lambda () (k (car ys))))
                                              (loop (cdr ys)))))))))
                         (run-amb
                          (lambda (thunk)
                            (handle-amb (lambda () (list (thunk)))))))
                  ;; 1. Pure state computation: accumulate 10 + 20 + 5 = 35
                  (let ((r-state
                         (run-state 10
                           (lambda ()
                             (let ((a (get)))
                               (put (+ a 20))
                               (let ((b (get)))
                                 (put (+ b 5))
                                 (list a b (get)))))))
                        ;; 2. Stacked State inside Amb: each nondeterministic branch gets isolated state!
                        (r-stacked
                         (run-amb
                           (lambda ()
                             (run-state 0
                               (lambda ()
                                 (let ((x (choose '(1 2))))
                                   (put (+ (get) x))
                                   (let ((y (choose '(10 20))))
                                     (put (+ (get) y))
                                     (list x y (get))))))))))
                    (list r-state r-stacked)))))
            "#
        ),
        "(((10 30 35) . 35) (((1 10 11) . 11) ((1 20 21) . 21) ((2 10 12) . 12) ((2 20 22) . 22)))"
    );
}

#[test]
fn tail_recursive_prompt_abort_loop() {
    // 5,000 sequential shift/reset aborts inside a single prompt must not blow the Rust stack
    assert_eq!(
        eval(
            r#"
            (reset
              (let loop ((n 0) (acc 0))
                (if (= n 5000)
                    acc
                    (shift k (loop (+ n 1) (+ acc 1))))))
            "#
        ),
        "5000"
    );
}

#[test]
fn top_level_default_prompt_and_handler_edge_cases() {
    // Implicit top-level default prompt handles `shift` and `abort-current-continuation`
    assert_eq!(eval("(+ 1 (shift k (+ 10 (k 5))))"), "16");
    assert_eq!(
        eval("(+ 999 (abort-current-continuation (default-continuation-prompt-tag) (lambda () 42)))"),
        "42"
    );

    // Default handler arity and type errors (both inside explicit prompt and at top level)
    let arity_err = eval_err("(reset (abort-current-continuation (default-continuation-prompt-tag) 1 2))");
    assert!(
        arity_err.contains("Default prompt handler expects 1 thunk argument"),
        "Unexpected error: {arity_err}"
    );
    let type_err = eval_err("(abort-current-continuation (default-continuation-prompt-tag) 42)");
    assert!(
        type_err.contains("Default prompt handler expects a procedure"),
        "Unexpected error: {type_err}"
    );

    // `#f` as the 3rd argument selects the default prompt handler (Racket parity)
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'df)))
              (call-with-continuation-prompt
                (lambda ()
                  (+ 10 (abort-current-continuation tag (lambda () 42))))
                tag
                #f))
            "#
        ),
        "42"
    );

    // Nested same-tag prompts where inner custom handler aborts to outer prompt
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'same)))
              (call-with-continuation-prompt
                (lambda ()
                  (call-with-continuation-prompt
                    (lambda () (abort-current-continuation tag 1))
                    tag
                    (lambda (v) (abort-current-continuation tag (+ v 10)))))
                tag
                (lambda (v) (+ v 100))))
            "#
        ),
        "111"
    );

    // Composable continuations captured in operator position (`Apply`) and 5-arg heap calls (`ArgEval`)
    assert_eq!(
        eval("(reset ((shift k (list (k +) (k -) (k *))) 10 3))"),
        "(13 7 30)"
    );
    assert_eq!(
        eval("(reset (list 1 2 (shift k (list (k 30) (k 300))) 4 5))"),
        "((1 2 30 4 5) (1 2 300 4 5))"
    );

    // Passing an undelimited `call/cc` continuation or composable continuation as a prompt handler
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'esc)))
              (+ 1 (call/cc
                     (lambda (exit)
                       (+ 100 (call-with-continuation-prompt
                                (lambda () (abort-current-continuation tag 41))
                                tag
                                exit))))))
            "#
        ),
        "42"
    );
    assert_eq!(
        eval(
            r#"
            (let ((tag (make-continuation-prompt-tag 'comp))
                  (add10 (reset (+ 10 (shift k k)))))
              (call-with-continuation-prompt
                (lambda () (abort-current-continuation tag 32))
                tag
                add10))
            "#
        ),
        "42"
    );
}

