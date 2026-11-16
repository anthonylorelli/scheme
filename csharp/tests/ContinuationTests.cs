using System;
using System.IO;
using Xunit;

namespace Scheme.Tests;

public class ContinuationTests : IClassFixture<InterpreterFixture>
{
    private readonly InterpreterFixture _fx;

    public ContinuationTests(InterpreterFixture fx)
    {
        _fx = fx;
    }

    private static string RepoRoot =>
        Path.GetFullPath(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "..", "..", "..", "..", ".."));

    [Fact]
    public void FirstClassValue_UpwardEscapeAndDataStructures()
    {
        Assert.Equal(Scheme.Boolean.TRUE, _fx.EvalLast("(call/cc (lambda (k) (procedure? k)))"));

        // Stored in vector and pair, verified eq?, then invoked
        Assert.Equal(142L, _fx.EvalLast(@"
            (let ((vec (vector #f))
                  (cell (cons #f '())))
              (let ((n (+ 100 (call/cc
                               (lambda (k)
                                 (vector-set! vec 0 k)
                                 (set-car! cell k)
                                 5)))))
                (if (= n 105)
                    (if (eq? (vector-ref vec 0) (car cell))
                        ((vector-ref vec 0) 42)
                        -1)
                    n)))"));

        // Upward continuation escaping the function frame that captured it
        Assert.Equal(1234L, _fx.EvalLast(@"
            (let ((make-adder-hole
                   (lambda (base)
                     (let ((saved #f))
                       (let ((sum (+ base (call/cc (lambda (k) (set! saved k) 0)))))
                         (list sum saved))))))
              (let ((res (make-adder-hole 1000)))
                (if (= (car res) 1000)
                    ((cadr res) 234)
                    (car res))))"));

        // Higher-order application via `apply` and canonical R5RS identifier
        Assert.Equal(42L, _fx.EvalLast(
            "(apply call-with-current-continuation (list (lambda (k) (+ 99 (apply k '(42))))))"));
    }

    [Fact]
    public void MultiShot_AcrossAllEvaluationContexts()
    {
        // CondContext: taking BOTH branches of the same `if` expression
        Assert.Equal("(consequent alternate)", _fx.EvalLast(@"
            (let ((saved #f) (branches '()))
              (let ((r (if (call/cc (lambda (k) (set! saved k) #t))
                           'consequent
                           'alternate)))
                (set! branches (cons r branches))
                (if (null? (cdr branches))
                    (saved #f)
                    (reverse branches))))")!.ToString());

        // AssignContext (`set!`) and DefineContext (`letrec`)
        Assert.Equal("(10 20 30)", _fx.EvalLast(@"
            (let ((x 0) (saved #f) (log '()))
              (set! x (call/cc (lambda (k) (set! saved k) 10)))
              (set! log (cons x log))
              (if (< x 30)
                  (saved (+ x 10))
                  (reverse log)))")!.ToString());

        Assert.Equal("(50 60 70)", _fx.EvalLast(@"
            (let ((saved #f) (log '()))
              (letrec ((x (call/cc (lambda (k) (set! saved k) 5))))
                (set! log (cons (* x 10) log))
                (if (< x 7)
                    (saved (+ x 1))
                    (reverse log))))")!.ToString());

        // SeqContext (`begin`) with two nested captures resumed out of order
        Assert.Equal("(a1 b1 done b2 done a2 b1 done)", _fx.EvalLast(@"
            (let ((k1 #f) (k2 #f) (trace '()) (step 0))
              (begin
                (let ((a (call/cc (lambda (k) (set! k1 k) 'a1))))
                  (set! trace (cons a trace)))
                (let ((b (call/cc (lambda (k) (set! k2 k) 'b1))))
                  (set! trace (cons b trace)))
                (set! trace (cons 'done trace)))
              (set! step (+ step 1))
              (cond
                ((= step 1) (k2 'b2))
                ((= step 2) (k1 'a2))
                (else (reverse trace))))")!.ToString());

        // Multi-operand ArgContext / RibContext
        Assert.Equal("((1 20 300) (2 20 300) (1 40 300) (1 20 600))", _fx.EvalLast(@"
            (let ((k1 #f) (k2 #f) (k3 #f) (stage 0) (acc '()))
              (let ((triple (list
                             (call/cc (lambda (k) (if (not k1) (set! k1 k)) 1))
                             (call/cc (lambda (k) (if (not k2) (set! k2 k)) 20))
                             (call/cc (lambda (k) (if (not k3) (set! k3 k)) 300)))))
                (set! acc (cons triple acc))
                (set! stage (+ stage 1))
                (cond
                  ((= stage 1) (k1 2))
                  ((= stage 2) (k2 40))
                  ((= stage 3) (k3 600))
                  (else (reverse acc)))))")!.ToString());

        // FuncContext & ApplyContext: captures in both operator and operand positions
        Assert.Equal("(125 75 2500 107)", _fx.EvalLast(@"
            (let ((k-op #f) (k-arg #f) (stage 0) (out '()))
              (let ((v ((call/cc (lambda (k) (if (not k-op) (set! k-op k)) +))
                        100
                        (call/cc (lambda (k) (if (not k-arg) (set! k-arg k)) 25)))))
                (set! out (cons v out))
                (set! stage (+ stage 1))
                (cond
                  ((= stage 1) (k-op -))
                  ((= stage 2) (k-op *))
                  ((= stage 3) (k-arg 7))
                  (else (reverse out)))))")!.ToString());

        // MultArgContext (`call-with-values`) with 0 values and multiple values
        Assert.Equal("zero-ok", _fx.EvalLast(
            "(call-with-values (lambda () (call/cc (lambda (k) (k)))) (lambda () 'zero-ok))")!.ToString());

        Assert.Equal("(6 60)", _fx.EvalLast(@"
            (let ((saved #f) (acc '()))
              (let ((s (call-with-values
                         (lambda () (call/cc (lambda (k) (set! saved k) (values 1 2 3))))
                         (lambda (a b c) (+ a b c)))))
                (set! acc (cons s acc))
                (if (< s 60)
                    (saved 10 20 30)
                    (reverse acc))))")!.ToString());

        // Capturing across `apply`, across `eval`, and inside another continuation's argument
        Assert.Equal("(1033 1100)", _fx.EvalLast(@"
            (let ((saved #f) (res '()))
              (let ((v (+ 1000 (apply (lambda (a b c)
                                        (+ a b (call/cc (lambda (k) (set! saved k) c))))
                                      '(10 20 3)))))
                (set! res (cons v res))
                (if (= v 1033) (saved 70) (reverse res))))")!.ToString());

        Assert.Equal(42L, _fx.EvalLast("(+ 40 (eval '(call/cc (lambda (k) (+ 999 (k 2))))))"));

        Assert.Equal("(17 35)", _fx.EvalLast(@"
            (let ((inner-k #f) (seen '()))
              (let ((v (+ 10 (call/cc
                              (lambda (outer-k)
                                (+ 999
                                   (outer-k
                                    (+ 5 (call/cc (lambda (k2) (set! inner-k k2) 2))))))))))
                (set! seen (cons v seen))
                (if (null? (cdr seen))
                    (inner-k 20)
                    (reverse seen))))")!.ToString());
    }

    [Fact]
    public void LexicalClosures_FreshFramesPerReentryAndSharedMutation()
    {
        Assert.Equal("((1 300) (2 300) (3 300))", _fx.EvalLast(@"
            (let ((saved-k #f)
                  (getters '())
                  (shared 0))
              (let ((x (call/cc (lambda (k) (set! saved-k k) 1))))
                (set! shared (+ shared 100))
                (set! getters (cons (lambda () (list x shared)) getters))
                (if (< x 3)
                    (saved-k (+ x 1))
                    (let loop ((fs (reverse getters)) (acc '()))
                      (if (null? fs)
                          (reverse acc)
                          (loop (cdr fs) (cons ((car fs)) acc)))))))")!.ToString());
    }

    [Fact]
    public void SelfApplication_CallCcCallCcAndYinYangPuzzle()
    {
        Assert.Equal("self-applied", _fx.EvalLast(@"
            (let ((k (call/cc call/cc)))
              (if (procedure? k)
                  (k 'self-applied)
                  k))")!.ToString());

        Assert.Equal("direct-ok", _fx.EvalLast("((call/cc call/cc) (lambda (k) 'direct-ok))")!.ToString());

        Assert.Equal("*.*..*...*....*.....*", _fx.EvalLast(@"
            (let ((out '()) (steps 0))
              (call/cc
               (lambda (exit)
                 (let* ((yin
                         ((lambda (cc)
                            (set! out (cons #\* out))
                            (set! steps (+ steps 1))
                            (if (= steps 21) (exit (apply string (reverse out))))
                            cc)
                          (call/cc (lambda (c) c))))
                        (yang
                         ((lambda (cc)
                            (set! out (cons #\. out))
                            (set! steps (+ steps 1))
                            (if (= steps 21) (exit (apply string (reverse out))))
                            cc)
                          (call/cc (lambda (c) c)))))
                   (yin yang)))))")!.ToString());
    }

    [Fact]
    public void AmbBacktracking_FourQueensSolutions()
    {
        Assert.Equal("((2 4 1 3) (3 1 4 2))", _fx.EvalLast(@"
            (let ((amb-fail (lambda () 'exhausted)))
              (letrec ((choose
                        (lambda (ls)
                          (if (null? ls)
                              (amb-fail)
                              (call/cc
                               (lambda (cc)
                                 (let ((prev amb-fail))
                                   (set! amb-fail
                                         (lambda ()
                                           (set! amb-fail prev)
                                           (cc (choose (cdr ls)))))
                                   (car ls)))))))
                       (require
                        (lambda (ok)
                          (if (not ok) (choose '()))))
                       (bag-of
                        (lambda (thunk)
                          (let ((prev amb-fail) (acc '()))
                            (call/cc
                             (lambda (done)
                               (set! amb-fail
                                     (lambda ()
                                       (set! amb-fail prev)
                                       (done (reverse acc))))
                               (let ((v (thunk)))
                                 (set! acc (cons v acc))
                                 (amb-fail)))))))
                       (abs-diff
                        (lambda (a b) (if (> a b) (- a b) (- b a))))
                       (safe?
                        (lambda (col placed dist)
                          (if (null? placed)
                              #t
                              (let ((other (car placed)))
                                (if (or (= col other) (= (abs-diff col other) dist))
                                    #f
                                    (safe? col (cdr placed) (+ dist 1))))))))
                (bag-of
                 (lambda ()
                   (let* ((q1 (choose '(1 2 3 4)))
                          (q2 (choose '(1 2 3 4))))
                     (require (safe? q2 (list q1) 1))
                     (let ((q3 (choose '(1 2 3 4))))
                       (require (safe? q3 (list q2 q1) 1))
                       (let ((q4 (choose '(1 2 3 4))))
                         (require (safe? q4 (list q3 q2 q1) 1))
                         (list q1 q2 q3 q4))))))))")!.ToString());
    }

    [Fact]
    public void DualStackGenerators_SameFringeWithLazyShortCircuit()
    {
        Assert.Equal("(#t #f (1 1 2 2 99 3))", _fx.EvalLast(@"
            (letrec ((make-iter
                      (lambda (tree on-visit)
                        (let ((caller #f) (resume #f))
                          (letrec ((yield-leaf
                                    (lambda (leaf)
                                      (call/cc
                                       (lambda (k)
                                         (set! resume k)
                                         (caller leaf)))))
                                   (walk
                                    (lambda (n)
                                      (cond
                                        ((null? n) 'skip)
                                        ((pair? n) (walk (car n)) (walk (cdr n)))
                                        (else (on-visit n) (yield-leaf n))))))
                            (set! resume (lambda (ig) (walk tree) (caller 'eof)))
                            (lambda ()
                              (call/cc (lambda (k) (set! caller k) (resume 'go))))))))
                     (same-fringe?
                      (lambda (t1 t2 on-visit)
                        (let ((i1 (make-iter t1 on-visit))
                              (i2 (make-iter t2 on-visit)))
                          (let loop ()
                            (let* ((x (i1)) (y (i2)))
                              (cond
                                ((and (eq? x 'eof) (eq? y 'eof)) #t)
                                ((equal? x y) (loop))
                                (else #f))))))))
              (let ((visited '()))
                (let ((match1 (same-fringe? '(((1 . 2) . 3) . (4 . 5))
                                            '(1 . (2 . (3 . (4 . 5))))
                                            (lambda (x) #f)))
                      (match2 (same-fringe? '((1 . 2) . (99 . (4 . 5)))
                                            '((1 . (2 . 3)) . (4 . 5))
                                            (lambda (x) (set! visited (cons x visited))))))
                  (list match1 match2 (reverse visited)))))")!.ToString());
    }

    [Fact]
    public void CooperativeThreads_RoundRobinSpawnAndYield()
    {
        Assert.Equal("(a1 b1 a2 b2 a3 b3)", _fx.EvalLast(@"
            (let ((q '()) (done-k #f) (trace '()))
              (letrec ((enq! (lambda (t) (set! q (append q (list t)))))
                       (deq! (lambda () (let ((h (car q))) (set! q (cdr q)) h)))
                       (yield!
                        (lambda ()
                          (call/cc
                           (lambda (k)
                             (enq! (lambda () (k 'ok)))
                             ((deq!))))))
                       (spawn!
                        (lambda (thunk)
                          (enq!
                           (lambda ()
                             (thunk)
                             (if (null? q)
                                 (done-k (reverse trace))
                                 ((deq!)))))))
                       (emit! (lambda (x) (set! trace (cons x trace)))))
                (spawn! (lambda () (emit! 'a1) (yield!) (emit! 'a2) (yield!) (emit! 'a3)))
                (spawn! (lambda () (emit! 'b1) (yield!) (emit! 'b2) (yield!) (emit! 'b3)))
                (call/cc (lambda (exit) (set! done-k exit) ((deq!))))))")!.ToString());
    }

    [Fact]
    public void DynamicWind_StateSpaceRerootingOutwardAndInward()
    {
        Assert.Equal(
            "(enter-1 enter-2 exit-2 exit-1 escaped enter-1 enter-2 reentered exit-2 exit-1 completed)",
            _fx.EvalLast(@"
            (let ((winders '()) (log '()) (inward-k #f))
              (letrec ((len (lambda (ls) (if (null? ls) 0 (+ 1 (len (cdr ls))))))
                       (drop-n (lambda (ls n) (if (= n 0) ls (drop-n (cdr ls) (- n 1)))))
                       (common-tail
                        (lambda (x y)
                          (let ((lx (len x)) (ly (len y)))
                            (let loop ((x (if (> lx ly) (drop-n x (- lx ly)) x))
                                       (y (if (> ly lx) (drop-n y (- ly lx)) y)))
                              (if (eq? x y) x (loop (cdr x) (cdr y)))))))
                       (reroot!
                        (lambda (target)
                          (let ((tail (common-tail target winders)))
                            (let unwind ((ls winders))
                              (if (not (eq? ls tail))
                                  (begin
                                    (set! winders (cdr ls))
                                    ((cdar ls))
                                    (unwind (cdr ls)))))
                            (let rewind ((ls target))
                              (if (not (eq? ls tail))
                                  (begin
                                    (rewind (cdr ls))
                                    ((caar ls))
                                    (set! winders ls)))))))
                       (dw-call/cc
                        (lambda (proc)
                          (call/cc
                           (lambda (k)
                             (let ((saved winders))
                               (proc (lambda (v)
                                       (if (not (eq? saved winders)) (reroot! saved))
                                       (k v))))))))
                       (dyn-wind
                        (lambda (before thunk after)
                          (before)
                          (set! winders (cons (cons before after) winders))
                          (let ((r (thunk)))
                            (set! winders (cdr winders))
                            (after)
                            r)))
                       (rec! (lambda (e) (set! log (cons e log)))))
                (let ((v (dw-call/cc
                          (lambda (esc)
                            (dyn-wind
                             (lambda () (rec! 'enter-1))
                             (lambda ()
                               (dyn-wind
                                (lambda () (rec! 'enter-2))
                                (lambda ()
                                  (let ((rv (dw-call/cc
                                             (lambda (k)
                                               (set! inward-k k)
                                               (esc 'escaped)))))
                                    (rec! rv)
                                    'completed))
                                (lambda () (rec! 'exit-2))))
                             (lambda () (rec! 'exit-1)))))))
                  (rec! v)
                  (if inward-k
                      (let ((k inward-k))
                        (set! inward-k #f)
                        (k 'reentered))
                      (reverse log)))))")!.ToString());
    }

    [Fact]
    public void FilinskiShiftReset_DelimitedContinuationsViaCallCc()
    {
        Assert.Equal("(41 (11 21 12 22 13 23))", _fx.EvalLast(@"
            (let ((meta-k (lambda (v) v)))
              (letrec ((abort (lambda (thunk) (let ((v (thunk))) (meta-k v))))
                       (reset
                        (lambda (thunk)
                          (call/cc
                           (lambda (k)
                             (let ((prev meta-k))
                               (set! meta-k (lambda (v) (set! meta-k prev) (k v)))
                               (abort thunk))))))
                       (shift
                        (lambda (f)
                          (call/cc
                           (lambda (k)
                             (abort
                              (lambda ()
                                (f (lambda (v)
                                     (reset (lambda () (k v))))))))))))
                (let ((r1 (+ 1 (reset (lambda () (* 2 (shift (lambda (k) (k (k 10)))))))))
                      (r2 (reset
                           (lambda ()
                             (let* ((x (shift (lambda (k) (append (k 1) (k 2) (k 3)))))
                                    (y (shift (lambda (k) (append (k 10) (k 20))))))
                               (list (+ x y)))))))
                  (list r1 r2))))")!.ToString());
    }

    [Fact]
    public void DeepStackReconstruction_AndTailCallFrameInvariance()
    {
        Assert.Equal("(50 150 0)", _fx.EvalLast(@"
            (let ((deep-k #f) (out '()) (runs 0))
              (letrec ((climb
                        (lambda (n)
                          (if (= n 0)
                              (call/cc (lambda (k) (set! deep-k k) 0))
                              (+ 1 (climb (- n 1)))))))
                (let ((v (climb 50)))
                  (set! out (cons v out))
                  (set! runs (+ runs 1))
                  (cond
                    ((= runs 1) (deep-k 100))
                    ((= runs 2) (deep-k -50))
                    (else (reverse out))))))")!.ToString());

        Assert.Equal("(5001 5200 0)", _fx.EvalLast(@"
            (let ((saved #f) (out '()) (runs 0))
              (letrec ((burn
                        (lambda (n acc)
                          (if (= n 0)
                              (+ acc (call/cc (lambda (k) (set! saved k) 1)))
                              (burn (- n 1) (+ acc 1))))))
                (let ((v (burn 5000 0)))
                  (set! out (cons v out))
                  (set! runs (+ runs 1))
                  (cond
                    ((= runs 1) (saved 200))
                    ((= runs 2) (saved -5000))
                    (else (reverse out))))))")!.ToString());
    }

    [Fact]
    public void SharedContinuationsScm_PassesAllAssertions()
    {
        var sw = new StringWriter();
        var origOut = Console.Out;
        Console.SetOut(sw);
        try
        {
            _fx.Interpreter.ReadFile(Path.Combine(RepoRoot, "test", "continuations.scm"));
        }
        finally
        {
            Console.SetOut(origOut);
        }

        string output = sw.ToString();
        Assert.DoesNotContain("FAIL [", output);
        Assert.Contains("Continuation tests passed: 32, failed: 0", output);
    }
}
