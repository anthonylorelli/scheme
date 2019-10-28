(begin
  (display (eval '(and 1 2 3))) (newline)
  (display (eval '(or (if #t #f #f) #f 1))) (newline)
  (display (eval '(let ((x 1)) x))) (newline)
  (sc-expand '(define-syntax try
                (syntax-rules ()
		  ((try exp (catch name handler))
		   (lambda ()
		     (call/cc
		      (lambda (k)
			(let ((saved-handlers *handlers))
			  (set! *handlers* (cons (list name k handler)
						 *handlers*))
			  exp
			  (set! *handlers* saved-handlers)))))))))
  (sc-expand '(define throw
		(lambda (exception)
		  (let ((handler-record (assq exception *handlers*)))
		    (if handler-record
			(let ((error-k (cadr handler-record))
			      (error-handler (caddr handler-record)))
			  (error-k (error-handler exception)))
			"Error")))))
  (sc-expand '(define throw
		(lambda (exception)
		  (let ((handler-record (assq exception *handlers*)))
		    (if handler-record
			(let ((error-k (cadr handler-record))
			      (error-handler (caddr handler-record)))
			  (error-k (error-handler exception)))
			"Error")))))
  (display (sc-expand '(define make-fibo-coroutine
						 (lambda (limit consumer-cor)
						   (coroutine no-init
                             (let fibo* ((curr 1) (prev 0))
							   (let ((new-val (+ curr prev)))
								 (if (> new-val limit)
									 (resume consumer-cor 'done)
									 (begin
									   (resume consume-cor new-val)
									   (fibo* new-val curr))))))))))
  (newline))




