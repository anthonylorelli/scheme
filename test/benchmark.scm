;;;
;;; Copyright 2006 Anthony J. Lorelli
;;; $Id: benchmark.scm 316 2006-03-29 13:37:29Z  $
;;;

(define safe-div
  (lambda (x y)
	(if (= y 0) (throw 'divide-by-zero) (/ x y))))

(define test-driver
  (try
   (safe-div 3 0)
   (catch 'divide-by-zero
	  (lambda (exception) "Divide-by-zero error"))))

(define make-fibo-coroutine
  (lambda (limit consumer-cor)
    (coroutine no-init
      (let fibo* ((curr 1) (prev 0))
	(let ((new-val (+ curr prev)))
	  (if (> new-val limit)
	      (resume consumer-cor 'done)
	      (begin
		(resume consumer-cor new-val)
		(fibo* new-val curr))))))))

(define make-list-coroutine
  (lambda (producer-cor)
    (coroutine no-initial-val
      (let loop ()
	(let ((val (resume producer-cor 'get-next-val)))
	  (if (eq? val 'done)
	      '()
	      (cons val (loop))))))))

(define get-fibo-list
  (lambda (limit)
    (letrec ((fibo-cor
	      (make-fibo-coroutine 
	       limit
	       (lambda (v) (list-cor v))))
	     (list-cor
	      (make-list-coroutine
	       (lambda (v) (fibo-cor v)))))
      (list-cor 'start))))

(define tak
  (lambda (x y z)
    (if (not (< y x))
	z
	(tak (tak (- x 1) y z)
	     (tak (- y 1) z x)
	     (tak (- z 1) x y)))))

(define ctak
  (lambda (x y z)
    (call/cc
     (lambda (k)
       (ctak+k k x y z)))))

(define ctak+k
  (lambda (k x y z)
    (cond ((not (< y x))
		   (k z))
		  (else
		   (call/cc
			(lambda (k)
			  (ctak+k
			   k
			   (call/cc
				(lambda (k) (ctak+k k (- x 1) y z)))
			   (call/cc
				(lambda (k) (ctak+k k (- y 1) z x)))
			   (call/cc
				(lambda (k) (ctak+k k (- z 1) x y))))))))))

(define capture-k
  (lambda (limit)
    (letrec ((capture-k*
	      (lambda (n)
		(if (< n limit)
		    (begin
		      (call/cc (lambda (k) 'ignored))
		      (capture-k* (+ n 1)))
		    'done))))
      (capture-k* 0))))

(define invoke-k
  (lambda (limit)
    (let ((n 0)
		  (loop 0))
      (call/cc (lambda (k) (set! loop k)))
      (if (< n limit)
		  (begin (set! n (+ n 1))
				 (loop 'ignored)))
		  'done)))
