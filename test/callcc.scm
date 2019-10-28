(load "../test/coroutines.scm")
(load "../test/exceptions.scm")
(load "../test/benchmark.scm")

(run-benchmark "exception" 100 (lambda () (test-driver)) #t)
(run-benchmark "coroutine" 100 (lambda () (get-fibo-list 2000)) #t)
(run-benchmark "tak" 10 (lambda () (tak 18 12 6)) #t)
(run-benchmark "ctak" 10 (lambda () (ctak 18 12 6)) #t)
(run-benchmark "capture-k" 100 (lambda () (capture-k 1000)) #t)
(run-benchmark "invoke-k" 100 (lambda () (invoke-k 1000)) #t)
