;;;
;;; Shared end-to-end Scheme verification suite for both C# and Rust interpreters
;;;

(define *passed* 0)
(define *failed* 0)

(define assert-equal
  (lambda (name actual expected)
    (if (equal? actual expected)
        (set! *passed* (+ *passed* 1))
        (begin
          (set! *failed* (+ *failed* 1))
          (display "FAIL [")
          (display name)
          (display "]: expected ")
          (display expected)
          (display ", got ")
          (display actual)
          (newline)))))

;; 1. Arithmetic & predicates
(assert-equal "add" (+ 17 25) 42)
(assert-equal "sub" (- 50 8) 42)
(assert-equal "neg-lit" (+ -10 52) 42)
(assert-equal "div" (/ 84 2) 42)
(assert-equal "num-eq-t" (= 7 7 7) #t)
(assert-equal "num-eq-f" (= 7 7 8) #f)
(assert-equal "lt" (< 3 5) #t)
(assert-equal "gt" (> 9 4) #t)
(assert-equal "zero-t" (zero? 0) #t)
(assert-equal "zero-f" (zero? 5) #f)
(assert-equal "number-pred" (list (number? 42) (number? "42")) '(#t #f))

;; 2. Booleans & truthiness
(assert-equal "bool-pred" (list (boolean? #t) (boolean? #f) (boolean? 0) (boolean? '())) '(#t #t #f #f))
(assert-equal "truthiness" (list (if 0 'yes 'no) (if '() 'yes 'no) (if #f 'yes 'no)) '(yes yes no))
(assert-equal "and-or" (list (and 1 2 3) (and 1 #f 3) (or #f 99 100)) '(3 #f 99))

;; 3. Pairs, lists, mutation
(assert-equal "cons-car-cdr" (let ((p (cons 'a 'b))) (list (pair? p) (car p) (cdr p))) '(#t a b))
(assert-equal "null-pair" (list (null? '()) (pair? '())) '(#t #f))
(assert-equal "set-car-cdr"
  (let ((p (cons 1 2)))
    (set-car! p 10)
    (set-cdr! p 20)
    p)
  '(10 . 20))
(assert-equal "append" (append '(1 2) '(3 4) '(5)) '(1 2 3 4 5))
(assert-equal "append-improper" (append '(a b) 'c) '(a b . c))
(assert-equal "reverse" (reverse '(1 2 3 4)) '(4 3 2 1))
(assert-equal "assq-memq"
  (list (assq 'b '((a 1) (b 2) (c 3))) (memq 'x '(w x y)))
  '((b 2) (x y)))

;; 4. Symbols, gensym, property lists
(assert-equal "case-fold" (eq? 'FooBar 'foobar) #t)
(assert-equal "gensym"
  (let ((g1 (gensym)) (g2 (gensym)))
    (list (symbol? g1) (gensym? g1) (gensym? 'regular) (eq? g1 g2)))
  '(#t #t #f #f))
(assert-equal "props"
  (let ((s (gensym)))
    (putprop s 'color 'blue)
    (let ((before (getprop s 'color)))
      (remprop s 'color)
      (list before (getprop s 'color))))
  '(blue #f))

;; 5. Characters & strings
(assert-equal "chars"
  (list (char? #\a) (char? #\space) (char? #\newline) (char? (integer->char 65)))
  '(#t #t #t #t))
(assert-equal "strings"
  (let ((s (string #\h #\i)))
    (list (string? s) (string-length s) (string=? s "hi") (string=? s "no")))
  '(#t 2 #t #f))

;; 6. Vectors
(assert-equal "vectors"
  (let ((v (make-vector 3 'init)))
    (vector-set! v 1 'mid)
    (list (vector? v) (vector-length v) (vector-ref v 0) (vector-ref v 1) (list->vector '(7 8))))
  '(#t 3 init mid #(7 8)))

;; 7. Closures, lexical scope, variadic lambdas, apply, map
(assert-equal "closure-counter"
  (let ((make-counter (lambda (n) (lambda (d) (set! n (+ n d)) n))))
    (let ((c (make-counter 10)))
      (let* ((r1 (c 5))
             (r2 (c 3)))
        (list r1 r2))))
  '(15 18))
(assert-equal "variadic-dotted" ((lambda (a b . rest) (list a b rest)) 1 2 3 4 5) '(1 2 (3 4 5)))
(assert-equal "variadic-symbol" ((lambda all all) 'x 'y 'z) '(x y z))
(assert-equal "apply-map" (map (lambda (x y) (+ x y)) '(1 2 3) '(10 20 30)) '(11 22 33))

;; 8. Macros (syntax-rules, syntax-case, quasiquote, define-macro) & eval
(define-syntax swap!
  (syntax-rules ()
    ((swap! a b)
     (let ((tmp a))
       (set! a b)
       (set! b tmp)))))

(assert-equal "hygienic-swap"
  (let ((tmp 1) (other 2))
    (swap! tmp other)
    (list tmp other))
  '(2 1))

(assert-equal "quasiquote"
  (let ((x 10) (xs '(20 30)))
    `(start ,x ,@xs end))
  '(start 10 20 30 end))

(eval '(define eval-defined-var 123))
(assert-equal "eval-top-level" (eval '(+ eval-defined-var 7)) 130)

;; 9. Multiple values
(assert-equal "values-multi"
  (call-with-values (lambda () (values 10 20 12)) (lambda (a b c) (+ a (+ b c))))
  42)
(assert-equal "values-zero"
  (call-with-values (lambda () (values)) (lambda () 'empty-ok))
  'empty-ok)

;; 10. Continuations across all context types & multi-shot resumption
(assert-equal "callcc-escape"
  (+ 100 (call/cc (lambda (k) (+ 1 (k 23)))))
  123)

(assert-equal "callcc-if-cond"
  (if (call/cc (lambda (k) (k #f))) 'bad 'good)
  'good)

(assert-equal "callcc-operator"
  (let ((saved #f)
        (seen '())
        (add10 (lambda (x) (+ x 10)))
        (sub3 (lambda (x) (- x 3))))
    (let ((res ((call/cc (lambda (k) (set! saved k) add10)) 5)))
      (set! seen (cons res seen))
      (if saved
          (let ((k saved))
            (set! saved #f)
            (k sub3))
          (reverse seen))))
  '(15 2))

(assert-equal "callcc-multishot"
  (let ((saved #f)
        (results '()))
    (let ((v (+ 10 (call/cc (lambda (k) (set! saved k) 1)))))
      (set! results (cons v results))
      (if (< (car results) 13)
          (saved (- v 9))
          (reverse results))))
  '(11 12 13))

;; 11. Coroutines, exceptions, tak, ctak, capture-k, invoke-k
(load "coroutines.scm")
(load "exceptions.scm")
(load "benchmark.scm")

(assert-equal "exception-benchmark" (test-driver) "Divide-by-zero error")
(assert-equal "coroutine-fibo"
  (get-fibo-list 2000)
  '(1 2 3 5 8 13 21 34 55 89 144 233 377 610 987 1597))
(assert-equal "tak" (tak 12 8 4) 5)
(assert-equal "ctak" (ctak 12 8 4) 5)
(assert-equal "capture-k" (capture-k 200) 'done)
(assert-equal "invoke-k" (invoke-k 200) 'done)

(display "Passed: ")
(display *passed*)
(display ", Failed: ")
(display *failed*)
(newline)
