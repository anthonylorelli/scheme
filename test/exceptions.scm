;;;
;;; Copyright 2006 Anthony J. Lorelli
;;; $Id: exceptions.scm 286 2006-03-13 22:34:56Z  $
;;;

(define *handlers* '())

(define-syntax try
  (syntax-rules ()
	((try exp (catch name handler))
	 (lambda ()
	   (call/cc
		(lambda (k)
		  (let ((saved-handlers *handlers*))
			(set! *handlers* (cons (list name k handler)
								   *handlers*))
			exp
			(set! *handlers* saved-handlers))))))))

(define throw
  (lambda (exception)
	(let ((handler-record (assq exception *handlers*)))
	  (if handler-record
		  (let ((error-k (cadr handler-record))
				(error-handler (caddr handler-record)))
			(error-k (error-handler exception)))
		  "Error"))))
