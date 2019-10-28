//
// Copyright 2006 Anthony J. Lorelli
// $Id: Primitives.cs 295 2006-03-18 23:04:06Z  $
//
namespace Scheme {
    using System.IO;

    public class Primitives {
        public static readonly StringReader Reader;

        static Primitives() {
            string s = @"

(define $sc-put-cte #f)
(define sc-expand #f)
(define $make-environment #f)
(define environment? #f)
(define interaction-environment #f)
(define identifier? #f)
(define datum->syntax-object #f)
(define syntax->list #f)
(define syntax-object->datum #f)
(define generate-temporaries #f)
(define free-identifier=? #f)
(define bound-identifier=? #f)
(define literal-identifier=? #f)
(define syntax-error #f)
(define $syntax-dispatch #f)

(define _expander
  (lambda (expr) expr))

(define eval
  (lambda (expr)
    (_native-eval 
      (_compile
        (_expander expr)))))

(define andmap
  (lambda (f first . rest)
    (if (null? first)
        #t
        (if (null? rest)
            (letrec ((andmap1
                       (lambda (first)
                         (letrec ((x (car first))
                                  (first (cdr first)))
                           (if (null? first)
                               (f x)
                               (if (f x) 
                                   (andmap1 first)
                                   #f))))))
              (andmap1 first))
            (letrec ((andmap-rest
                       (lambda (first rest)
                         (letrec ((x (car first))
                                  (xr (map car rest))
                                  (first (cdr first))
                                  (rest (map cdr rest)))
                           (if (null? first)
                               (apply f (cons x xr))
                               (if (apply f (cons x xr))
                                   (andmap-rest first rest)
                                   #f))))))
              (andmap-rest first rest))))))

(define caar
  (lambda (x)
    (car (car x))))

(define cadr
  (lambda (x)
    (car (cdr x))))

(define map
  (lambda (f ls . more)
    (if (null? more)
        (letrec ((map1 (lambda (ls)
                   (if (null? ls)
                       '()
                       (cons (f (car ls))
                             (map1 (cdr ls)))))))
          (map1 ls))
        (letrec ((map-more (lambda (ls more)
                             (if (null? ls)
                                 '()
                                 (cons (apply f (car ls) (map car more))
                                       (map-more (cdr ls)
                                                 (map cdr more)))))))
          (map-more ls more)))))

(define reverse
  (lambda (ls)
    (letrec ((rev
               (lambda (ls new)
                 (if (null? ls)
                     new
                     (rev (cdr ls) (cons (car ls) new))))))
      (rev ls '()))))

(define memv
  (lambda (x ls)
    (if (null? ls)
        #f
        (if (eqv? (car ls) x)
            ls
            (memv x (cdr ls))))))

(define memq
  (lambda (x ls)
    (if (null? ls)
        #f
        (if (eq? (car ls) x)
            ls
            (memq x (cdr ls))))))

(define assq
  (lambda (x ls)
    (if (null? ls)
        #f
        (if (eq? (caar ls) x)
            (car ls)
            (assq x (cdr ls))))))

;(define equal?
;  (lambda (x y)
;    (if (eqv? x y)
;        #t
;        (if (pair? x)
;            (and (pair? y)
;                 (equal? (car x) (car y))
;                 (equal? (cdr x) (cdr y)))
;            (if (string? x)
;                (and (string? y)
;                     (string=? x y))
;                (if (vector? x)
;                    (letrec ((n (vector-length x)))
;                      (and (= (vector-length y) n)
;                           (letrec ((loop 
;                                      (lambda (i)
;                                        (or (= i n)
;                                            (and (equal? (vector-ref x i) 
;                                                         (vector-ref y i))
;                                                 (loop (+ i 1)))))))
;                             (loop 0))))
;                    #f))))))

(define equal?
  (lambda (__%sym29 __%sym30)
    (if (eqv? __%sym29 __%sym30)
        #t
        (if (pair? __%sym29)
            (if (pair? __%sym30)
                (if (equal? (car __%sym29) (car __%sym30))
                    (equal? (cdr __%sym29) (cdr __%sym30))
                    #f)
                #f)
            (if (string? __%sym29)
                (if (string? __%sym30)
                    (string=? __%sym29 __%sym30)
                    (quote #f))
                (if (vector? __%sym29)
                    (letrec ((__%sym31 (vector-length __%sym29)))
                      (if (= (vector-length __%sym30) __%sym31)
                          (letrec ((__%sym32
                                    (lambda (__%sym33)
                                      ((lambda (__%sym34)
                                         (if __%sym34
                                             __%sym34
                                             (if (equal? (vector-ref __%sym29 __%sym33)
                                                         (vector-ref __%sym30 __%sym33))
                                                 (__%sym32 (+ __%sym33 (quote 1)))
                                                 (quote #f))))
                                       (= __%sym33 __%sym31)))))
                            (__%sym32 (quote 0))) (quote #f)))
                    (quote #f)))))))

(define not (lambda (x) (if x #f #t)))

            ";
            Reader = new StringReader(s);
        }
    }
}
