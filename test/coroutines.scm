;;;
;;; Copyright 2006 Anthony J. Lorelli
;;; $Id: coroutines.scm 277 2006-03-11 12:25:55Z  $
;;;

(define-macro coroutine
  (lambda (x . body)
    `(letrec ((local-control-state
	       (lambda (,x) ,@body))
	      (resume
	       (lambda (c v)
		 (call/cc
		  (lambda (k)
		    (set! local-control-state k)
		    (c v))))))
       (lambda (v)
	 (local-control-state v)))))
