;;;
;;; init.scm
;;;

(define call/cc call-with-current-continuation)

(set! _expander sc-expand)

; Already defined: cadr caar

(define cdar
  (lambda (ls) (cdr (car ls))))

(define cddr
  (lambda (ls) (cdr (cdr ls))))

(define caddr
  (lambda (ls) (car (cddr ls))))

(define time
  (lambda (proc)
    (let ((start (_get-ticks)))
      (let ((result (proc)))
        (let ((stop (_get-ticks)))
          (values result (- stop start)))))))

(define run-benchmark
  (lambda (name limit proc pred)
    (display "-------------------------------------------")
    (newline)
    (display "Benchmark: ") (display name) (newline)
    (display "Iterations: ") (display limit) (newline)
    (let ((total 0)
          (high 0)
          (low 999999999999))
      (let loop ((n 0))
        (if (< n limit)
            (begin
              (call-with-values (lambda () (time proc))
                (lambda (result ticks)
                  (display result) (newline)
                  (set! total (+ total ticks))
                  (if (< high ticks)
                      (set! high ticks))
                  (if (< ticks low)
                      (set! low ticks))))
              (loop (+ n 1)))
            (begin (set! total (/ total 10))
                   (display "Total: ")
                   (display total) (newline)
                   (display "Lowest: ")
                   (display (/ low 10)) (newline)
                   (display "Highest: ")
                   (display (/ high 10)) (newline)
                   (display "Average: ")
                   (display (/ total n))
                   (newline)))))))

(define newline
  (lambda ()
    (display (string #\newline))))

(define-syntax define-macro
  (lambda (x)
    (syntax-case x ()
      ((_ (name . args) . body)
       (syntax (define-macro name (lambda args . body))))
      ((_ name expander)
       (syntax
        (define-syntax name
          (lambda (y)
            (syntax-case y ()
              ((k . args)
               (datum->syntax-object (syntax k)
                 (apply expander
                        (cdr (syntax-object->datum y)))))))))))))
