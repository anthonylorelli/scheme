;;;
;;; Comprehensive First-Class Continuation Test Suite
;;; Exercises upward/downward extent, multi-shot re-entry across every AST
;;; context frame, (call/cc call/cc), Yin-Yang, McCarthy's amb backtracking,
;;; dual-stack same-fringe generators, cooperative round-robin threads, and
;;; user-space state-rerooting dynamic-wind.
;;;

(define *k-passed* 0)
(define *k-failed* 0)

(define check-k
  (lambda (name actual expected)
    (if (equal? actual expected)
        (set! *k-passed* (+ *k-passed* 1))
        (begin
          (set! *k-failed* (+ *k-failed* 1))
          (display "FAIL [")
          (display name)
          (display "]: expected ")
          (display expected)
          (display ", got ")
          (display actual)
          (newline)))))

;; ---------------------------------------------------------------------------
;; 1. First-class value properties & upward (escaping) extent
;; ---------------------------------------------------------------------------

(check-k "continuation-is-procedure"
  (call/cc (lambda (k) (procedure? k)))
  #t)

(check-k "continuation-in-data-structures"
  (let ((box (vector #f))
        (pair-cell (cons #f '())))
    (let ((x (+ 100 (call/cc
                     (lambda (k)
                       (vector-set! box 0 k)
                       (set-car! pair-cell k)
                       5)))))
      (if (= x 105)
          (begin
            (check-k "continuation-eq" (eq? (vector-ref box 0) (car pair-cell)) #t)
            ((vector-ref box 0) 42))
          x)))
  142)

;; Upward continuation returned out of a function whose frame has already exited
(define capture-adder-hole
  (lambda (base)
    (let ((captured #f))
      (let ((res (+ base (call/cc (lambda (k) (set! captured k) 0)))))
        (list res captured)))))

(check-k "upward-continuation-reentry"
  (let ((pair1 (capture-adder-hole 1000)))
    (let ((val1 (car pair1))
          (k (cadr pair1)))
      (if k
          ;; Re-entering `capture-adder-hole` after it already returned!
          ;; Clear `k` inside the resumed frame by passing 234; note `captured`
          ;; in the original environment still holds `k` unless we distinguish by `val1`.
          (if (= val1 1000)
              (k 234)
              val1)
          val1)))
  1234)

;; ---------------------------------------------------------------------------
;; 2. Multi-shot re-entry across EVERY interpreter Context frame type
;; ---------------------------------------------------------------------------

;; 2a. HaltContext (top-level continuation)
(define *halt-k* #f)
(define *halt-log* '())
(let ((v (call/cc (lambda (k) (set! *halt-k* k) 'first))))
  (set! *halt-log* (cons v *halt-log*)))
(if (< (car (list (if (eq? (car *halt-log*) 'first) 1 2))) 2)
    (*halt-k* 'second))
(check-k "context-halt-multishot" (reverse *halt-log*) '(first second))

;; 2b. CondContext: `(if <hole> consequent alternate)` taking BOTH branches!
(check-k "context-if-both-branches"
  (let ((saved #f)
        (taken '()))
    (let ((branch-result
           (if (call/cc (lambda (k) (set! saved k) #t))
               'then-branch
               'else-branch)))
      (set! taken (cons branch-result taken))
      (if (= (if (null? (cdr taken)) 1 2) 1)
          (saved #f)
          (reverse taken))))
  '(then-branch else-branch))

;; 2c. AssignContext: `(set! target <hole>)` resumed multiple times
(check-k "context-set-multishot"
  (let ((target 0)
        (saved #f)
        (history '()))
    (set! target (call/cc (lambda (k) (set! saved k) 10)))
    (set! history (cons target history))
    (if (< target 30)
        (saved (+ target 10))
        (reverse history)))
  '(10 20 30))

;; 2d. DefineContext: `(letrec ((x <hole>)) ...)` resumed multiple times
(check-k "context-define-letrec-multishot"
  (let ((saved #f)
        (history '()))
    (letrec ((x (call/cc (lambda (k) (set! saved k) 7))))
      (set! history (cons (* x 2) history))
      (if (< x 9)
          (saved (+ x 1))
          (reverse history))))
  '(14 16 18))

;; 2e. SeqContext: nested captures across sequential steps in `(begin ...)`
(check-k "context-begin-nested-captures"
  (let ((k1 #f)
        (k2 #f)
        (trace '()))
    (begin
      (let ((a (call/cc (lambda (k) (set! k1 k) 'a1))))
        (set! trace (cons a trace)))
      (let ((b (call/cc (lambda (k) (set! k2 k) 'b1))))
        (set! trace (cons b trace)))
      (set! trace (cons 'end trace)))
    (cond
      ((= (let loop ((ls trace) (n 0)) (if (null? ls) n (loop (cdr ls) (+ n 1)))) 3)
       ;; Resume k2 ONLY (skips step 1, re-runs step 2 and step 3)
       (k2 'b2))
      ((= (let loop ((ls trace) (n 0)) (if (null? ls) n (loop (cdr ls) (+ n 1)))) 5)
       ;; Now resume k1 (re-runs step 1, step 2 fresh with 'b1, and step 3)
       (k1 'a2))
      (else
       (reverse trace))))
  '(a1 b1 end b2 end a2 b1 end))

;; 2f. ArgContext / RibContext: multiple captures in the SAME argument list
(check-k "context-multi-operand-ribs"
  (let ((k-left #f)
        (k-mid #f)
        (k-right #f)
        (stage 0)
        (results '()))
    (let ((tuple (list
                  (call/cc (lambda (k) (if (not k-left) (set! k-left k)) 1))
                  (call/cc (lambda (k) (if (not k-mid) (set! k-mid k)) 20))
                  (call/cc (lambda (k) (if (not k-right) (set! k-right k)) 300)))))
      (set! results (cons tuple results))
      (set! stage (+ stage 1))
      (cond
        ((= stage 1) (k-left 2))      ;; Right & mid ribs preserved: (2 20 300)
        ((= stage 2) (k-mid 40))      ;; Right rib preserved, left re-evaluated to 1: (1 40 300)
        ((= stage 3) (k-right 600))   ;; Mid & left re-evaluated fresh: (1 20 600)
        (else (reverse results)))))
  '((1 20 300) (2 20 300) (1 40 300) (1 20 600)))

;; 2g. FuncContext & ApplyContext: capture in BOTH operator AND operand positions
(check-k "context-operator-and-operand-multishot"
  (let ((k-op #f)
        (k-arg #f)
        (stage 0)
        (out '()))
    (let ((val ((call/cc (lambda (k) (if (not k-op) (set! k-op k)) +))
                100
                (call/cc (lambda (k) (if (not k-arg) (set! k-arg k)) 25)))))
      (set! out (cons val out))
      (set! stage (+ stage 1))
      (cond
        ((= stage 1) (k-op -))    ;; Same evaluated args (100, 25), new operator `-` -> 75
        ((= stage 2) (k-op *))    ;; Same evaluated args (100, 25), new operator `*` -> 2500
        ((= stage 3) (k-arg 7))   ;; New arg 7, operator re-evaluated to `+` -> 107
        (else (reverse out)))))
  '(125 75 2500 107))

;; 2h. MultArgContext: `call-with-values` producer continuation invoked with 0 and multiple values
(check-k "context-call-with-values-multishot"
  (let ((saved #f)
        (acc '()))
    (let ((sum (call-with-values
                 (lambda ()
                   (call/cc (lambda (k) (set! saved k) (values 1 2 3))))
                 (lambda (a b c) (+ a b c)))))
      (set! acc (cons sum acc))
      (if (< sum 60)
          (saved 10 20 30)
          (reverse acc))))
  '(6 60))

(check-k "context-call-with-values-zero-args"
  (call-with-values
    (lambda () (call/cc (lambda (k) (k))))
    (lambda () 'zero-values-delivered))
  'zero-values-delivered)

;; 2i. Capturing across `apply` and `eval`
(check-k "capture-across-apply"
  (let ((saved #f)
        (res '()))
    (let ((v (+ 1000 (apply (lambda (a b c)
                              (+ a b (call/cc (lambda (k) (set! saved k) c))))
                            '(10 20 3)))))
      (set! res (cons v res))
      (if (= v 1033)
          (saved 70)
          (reverse res))))
  '(1033 1100))

(check-k "capture-across-eval"
  (+ 40 (eval '(call/cc (lambda (k) (+ 999 (k 2))))))
  42)

;; ---------------------------------------------------------------------------
;; 3. Mind-benders: (call/cc call/cc) and the Yin-Yang Puzzle
;; ---------------------------------------------------------------------------

(check-k "callcc-self-application-let"
  (let ((k (call/cc call/cc)))
    (if (procedure? k)
        (k 'self-applied-ok)
        k))
  'self-applied-ok)

(check-k "callcc-self-application-direct"
  ((call/cc call/cc) (lambda (k) 'direct-ok))
  'direct-ok)

;; Classic Yin-Yang puzzle bounded to 21 steps:
;; Stars appear at indices 1, 3, 6, 10, 15, 21 separated by 1, 2, 3, 4, 5 dots
(check-k "yin-yang-21-steps"
  (let ((out '())
        (steps 0))
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
         (yin yang)))))
  "*.*..*...*....*.....*")

;; ---------------------------------------------------------------------------
;; 4. McCarthy's `amb` Nondeterministic Backtracking
;; ---------------------------------------------------------------------------

(define *amb-fail* (lambda () 'no-more-choices))

(define amb-choose
  (lambda (choices)
    (if (null? choices)
        (*amb-fail*)
        (call/cc
         (lambda (cc)
           (let ((prev-fail *amb-fail*))
             (set! *amb-fail*
                   (lambda ()
                     (set! *amb-fail* prev-fail)
                     (cc (amb-choose (cdr choices)))))
             (car choices)))))))

(define amb-require
  (lambda (pred)
    (if (not pred) (amb-choose '()))))

(define amb-bag-of
  (lambda (thunk)
    (let ((prev-fail *amb-fail*)
          (results '()))
      (call/cc
       (lambda (done)
         (set! *amb-fail*
               (lambda ()
                 (set! *amb-fail* prev-fail)
                 (done (reverse results))))
         (let ((val (thunk)))
           (set! results (cons val results))
           (*amb-fail*)))))))

(define range
  (lambda (lo hi)
    (if (> lo hi)
        '()
        (cons lo (range (+ lo 1) hi)))))

;; Collect ALL Pythagorean triples (a, b, c) with 1 <= a <= b <= c <= 15
(check-k "amb-all-pythagorean-triples"
  (amb-bag-of
   (lambda ()
     (let* ((a (amb-choose (range 1 15)))
            (b (amb-choose (range a 15)))
            (c (amb-choose (range b 15))))
       (amb-require (= (+ (* a a) (* b b)) (* c c)))
       (list a b c))))
  '((3 4 5) (5 12 13) (6 8 10) (9 12 15)))

;; ---------------------------------------------------------------------------
;; 5. Dual-Stack Lazy Tree Generators (`same-fringe?` with short-circuiting)
;; ---------------------------------------------------------------------------

(define make-leaf-iterator
  (lambda (tree on-visit)
    (let ((caller #f)
          (resume-walk #f))
      (letrec ((yield-leaf
                (lambda (leaf)
                  (call/cc
                   (lambda (k)
                     (set! resume-walk k)
                     (caller leaf)))))
               (walk
                (lambda (node)
                  (cond
                    ((null? node) 'skip)
                    ((pair? node)
                     (walk (car node))
                     (walk (cdr node)))
                    (else
                     (on-visit node)
                     (yield-leaf node))))))
        (set! resume-walk
              (lambda (ignored)
                (walk tree)
                (caller 'eof)))
        (lambda ()
          (call/cc
           (lambda (k)
             (set! caller k)
             (resume-walk 'go))))))))

(define same-fringe?
  (lambda (t1 t2 on-visit)
    (let ((it1 (make-leaf-iterator t1 on-visit))
          (it2 (make-leaf-iterator t2 on-visit)))
      (let loop ()
        (let* ((x (it1))
               (y (it2)))
          (cond
            ((and (eq? x 'eof) (eq? y 'eof)) #t)
            ((equal? x y) (loop))
            (else #f)))))))

(check-k "same-fringe-matching-shapes"
  (same-fringe?
    '(((1 . 2) . (3 . 4)) . (5 . (6 . 7)))
    '(1 . (2 . ((3 . (4 . 5)) . (6 . 7))))
    (lambda (x) #f))
  #t)

(check-k "same-fringe-lazy-short-circuit"
  (let ((visited '()))
    (let ((ok (same-fringe?
                '((1 . 2) . (99 . (4 . 5)))
                '((1 . (2 . 3)) . (4 . 5))
                (lambda (leaf) (set! visited (cons leaf visited))))))
      ;; Must stop immediately upon seeing 99 vs 3 without visiting 4 or 5!
      (list ok (reverse visited))))
  '(#f (1 1 2 2 99 3)))

;; ---------------------------------------------------------------------------
;; 6. Cooperative Round-Robin Fiber Scheduler (`spawn` / `yield` / `run`)
;; ---------------------------------------------------------------------------

(define run-round-robin-test
  (lambda ()
    (let ((ready-q '())
          (scheduler-exit #f)
          (events '()))
      (letrec ((enqueue!
                (lambda (k)
                  (set! ready-q (append ready-q (list k)))))
               (dequeue!
                (lambda ()
                  (let ((next (car ready-q)))
                    (set! ready-q (cdr ready-q))
                    next)))
               (yield!
                (lambda ()
                  (call/cc
                   (lambda (k)
                     (enqueue! (lambda () (k 'resumed)))
                     ((dequeue!))))))
               (spawn!
                (lambda (thunk)
                  (enqueue!
                   (lambda ()
                     (thunk)
                     (if (null? ready-q)
                         (scheduler-exit (reverse events))
                         ((dequeue!)))))))
               (log-step!
                (lambda (tag)
                  (set! events (cons tag events)))))
        (spawn! (lambda () (log-step! 't1-1) (yield!) (log-step! 't1-2) (yield!) (log-step! 't1-3)))
        (spawn! (lambda () (log-step! 't2-1) (yield!) (log-step! 't2-2) (yield!) (log-step! 't2-3)))
        (spawn! (lambda () (log-step! 't3-1) (yield!) (log-step! 't3-2) (yield!) (log-step! 't3-3)))
        (call/cc
         (lambda (exit)
           (set! scheduler-exit exit)
           ((dequeue!))))))))

(check-k "cooperative-round-robin-threads"
  (run-round-robin-test)
  '(t1-1 t2-1 t3-1 t1-2 t2-2 t3-2 t1-3 t2-3 t3-3))

;; ---------------------------------------------------------------------------
;; 7. User-Space State-Space-Rerooting `dynamic-wind` on top of `call/cc`
;; ---------------------------------------------------------------------------

(define list-len
  (lambda (ls)
    (let loop ((ls ls) (n 0))
      (if (null? ls) n (loop (cdr ls) (+ n 1))))))

(define drop-n
  (lambda (ls n)
    (if (= n 0) ls (drop-n (cdr ls) (- n 1)))))

(define shared-suffix
  (lambda (x y)
    (let ((lx (list-len x))
          (ly (list-len y)))
      (let loop ((x (if (> lx ly) (drop-n x (- lx ly)) x))
                 (y (if (> ly lx) (drop-n y (- ly lx)) y)))
        (if (eq? x y)
            x
            (loop (cdr x) (cdr y)))))))

(check-k "dynamic-wind-outward-and-inward-reentry"
  (let ((winders '())
        (log '())
        (inward-k #f))
    (letrec ((record!
              (lambda (evt)
                (set! log (cons evt log))))
             (reroot!
              (lambda (target)
                (let ((tail (shared-suffix target winders)))
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
                             (if (not (eq? saved winders))
                                 (reroot! saved))
                             (k v))))))))
             (dyn-wind
              (lambda (before thunk after)
                (before)
                (set! winders (cons (cons before after) winders))
                (let ((res (thunk)))
                  (set! winders (cdr winders))
                  (after)
                  res))))
      (let ((out-val
             (dw-call/cc
              (lambda (escape)
                (dyn-wind
                 (lambda () (record! 'in-outer))
                 (lambda ()
                   (dyn-wind
                    (lambda () (record! 'in-inner))
                    (lambda ()
                      (record! 'body-start)
                      (let ((resumed-val
                             (dw-call/cc
                              (lambda (k)
                                (set! inward-k k)
                                (escape 'jumped-out)))))
                        (record! resumed-val)
                        'normal-finish))
                    (lambda () (record! 'out-inner))))
                 (lambda () (record! 'out-outer)))))))
        (record! out-val)
        (if inward-k
            (let ((k inward-k))
              (set! inward-k #f)
              (k 'jumped-back-in))
            (reverse log)))))
  '(in-outer in-inner body-start out-inner out-outer jumped-out
    in-outer in-inner jumped-back-in out-inner out-outer normal-finish))

;; ---------------------------------------------------------------------------
;; 8. 4-Queens Puzzle Solved via `amb` Chronological Backtracking
;; ---------------------------------------------------------------------------

(define abs-diff
  (lambda (a b)
    (if (> a b) (- a b) (- b a))))

(define safe-queen?
  (lambda (col placed dist)
    (if (null? placed)
        #t
        (let ((other (car placed)))
          (if (or (= col other)
                  (= (abs-diff col other) dist))
              #f
              (safe-queen? col (cdr placed) (+ dist 1)))))))

(check-k "amb-4-queens-all-solutions"
  (amb-bag-of
   (lambda ()
     (let* ((q1 (amb-choose '(1 2 3 4)))
            (q2 (amb-choose '(1 2 3 4))))
       (amb-require (safe-queen? q2 (list q1) 1))
       (let ((q3 (amb-choose '(1 2 3 4))))
         (amb-require (safe-queen? q3 (list q2 q1) 1))
         (let ((q4 (amb-choose '(1 2 3 4))))
           (amb-require (safe-queen? q4 (list q3 q2 q1) 1))
           (list q1 q2 q3 q4))))))
  '((2 4 1 3) (3 1 4 2)))

;; ---------------------------------------------------------------------------
;; 9. Deep Non-Tail Stack Reconstruction & Multi-Shot Replay (Depth 50)
;; ---------------------------------------------------------------------------

(check-k "deep-stack-reconstruction-multishot"
  (let ((deep-k #f)
        (outputs '()))
    (letrec ((climb
              (lambda (n)
                (if (= n 0)
                    (call/cc (lambda (k) (set! deep-k k) 0))
                    (+ 1 (climb (- n 1)))))))
      (let ((res (climb 50)))
        (set! outputs (cons res outputs))
        (cond
          ((= (list-len outputs) 1) (deep-k 100))
          ((= (list-len outputs) 2) (deep-k -50))
          (else (reverse outputs))))))
  '(50 150 0))

;; ---------------------------------------------------------------------------
;; 10. Nested Loop `break` and `continue` + Capture Inside Continuation Arg
;; ---------------------------------------------------------------------------

(check-k "nested-loop-break-and-continue"
  (let ((acc '()))
    (call/cc
     (lambda (outer-break)
       (let outer ((i 1))
         (if (< i 5)
             (begin
               (let inner ((j 1))
                 (if (< j 5)
                     (begin
                       (call/cc
                        (lambda (inner-continue)
                          (if (= j 2) (inner-continue 'skip-2))
                          (if (and (= i 3) (= j 3)) (outer-break 'stopped))
                          (set! acc (cons (list i j) acc))))
                       (inner (+ j 1)))))
               (outer (+ i 1)))))))
    (reverse acc))
  '((1 1) (1 3) (1 4) (2 1) (2 3) (2 4) (3 1)))

(check-k "capture-inside-continuation-argument"
  (let ((inner-k #f)
        (seen '()))
    (let ((v (+ 10 (call/cc
                    (lambda (outer-k)
                      (+ 999
                         (outer-k
                          (+ 5 (call/cc
                                (lambda (k2)
                                  (set! inner-k k2)
                                  2))))))))))
      (set! seen (cons v seen))
      (if (= (list-len seen) 1)
          ;; Invoking inner-k resumes `(+ 5 <hole>)` INSIDE `(outer-k <arg>)`,
          ;; which then re-invokes `outer-k` with `(+ 5 20) = 25` -> `(+ 10 25) = 35`!
          (inner-k 20)
          (reverse seen))))
  '(17 35))

;; ---------------------------------------------------------------------------
;; 11. R5RS Pitfalls: Fresh Lexical Frames per Re-entry & Tail-Call Invariance
;; ---------------------------------------------------------------------------

(check-k "higher-order-call-with-current-continuation"
  (apply call-with-current-continuation
         (list (lambda (k) (+ 100 (apply k '(42))))))
  42)

;; Each re-entry into `(let ((x (call/cc ...))) ...)` applies `(lambda (x) ...)`
;; freshly, so closures over `x` from earlier runs must NOT be clobbered!
(check-k "closure-fresh-lexical-frame-per-reentry"
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
                (loop (cdr fs) (cons ((car fs)) acc)))))))
  '((1 300) (2 300) (3 300)))

;; Capturing inside a deep tail-recursive loop (depth 5000) and re-entering
;; multiple times proves tail calls do not leak continuation frames.
(check-k "tail-call-capture-frame-invariance"
  (let ((saved #f)
        (results '()))
    (letrec ((tail-burn
              (lambda (n acc)
                (if (= n 0)
                    (+ acc (call/cc (lambda (k) (set! saved k) 1)))
                    (tail-burn (- n 1) (+ acc 1))))))
      (let ((v (tail-burn 5000 0)))
        (set! results (cons v results))
        (cond
          ((= (list-len results) 1) (saved 200))
          ((= (list-len results) 2) (saved -5000))
          (else (reverse results))))))
  '(5001 5200 0))

;; ---------------------------------------------------------------------------
;; 12. Filinski's Encoding of Delimited Continuations (`shift` / `reset`)
;;     Purely via First-Class Undelimited `call/cc` + One Mutable Cell
;; ---------------------------------------------------------------------------

(define *meta-k* (lambda (v) v))

(define abort-to-meta
  (lambda (thunk)
    (let ((v (thunk)))
      (*meta-k* v))))

(define call-with-reset
  (lambda (thunk)
    (call/cc
     (lambda (k)
       (let ((prev-meta *meta-k*))
         (set! *meta-k*
               (lambda (v)
                 (set! *meta-k* prev-meta)
                 (k v)))
         (abort-to-meta thunk))))))

(define call-with-shift
  (lambda (f)
    (call/cc
     (lambda (k)
       (abort-to-meta
        (lambda ()
          (f (lambda (v)
               (call-with-reset
                (lambda () (k v)))))))))))

(check-k "filinski-shift-reset-double-application"
  (+ 1 (call-with-reset
        (lambda ()
          (* 2 (call-with-shift
                (lambda (k)
                  (k (k 10))))))))
  41)

(check-k "filinski-shift-reset-list-monad"
  (call-with-reset
   (lambda ()
     (let* ((x (call-with-shift
                (lambda (k)
                  (append (k 1) (k 2) (k 3)))))
            (y (call-with-shift
                (lambda (k)
                  (append (k 10) (k 20))))))
       (list (+ x y)))))
  '(11 21 12 22 13 23))

(display "Continuation tests passed: ")
(display *k-passed*)
(display ", failed: ")
(display *k-failed*)
(newline)
