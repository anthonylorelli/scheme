using System;
using System.IO;
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Scheme.Tests;

public class InterpreterFixture
{
    public Interpreter Interpreter { get; }

    public InterpreterFixture()
    {
        Interpreter = new Interpreter(new StringReader(string.Empty));
        InterpreterShell.LoadRuntime(Interpreter, AppDomain.CurrentDomain.BaseDirectory);
    }

    public object? EvalLast(string code)
    {
        var compiler = new Compiler(new MacroExpander(
            new Parser(new Scanner(new StringReader(code))), Interpreter));
        object? compiled;
        object? result = null;
        var evalMethod = typeof(Interpreter).GetMethod(
            "Evaluate",
            System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic,
            null,
            new[] { typeof(object), typeof(Scheme.Environment) },
            null)!;

        while ((compiled = compiler.Compile()) != null)
        {
            result = evalMethod.Invoke(Interpreter, new[] { compiled, Interpreter.GlobalEnvironment });
        }
        return result;
    }
}

public class InterpreterTests : IClassFixture<InterpreterFixture>
{
    private readonly InterpreterFixture _fx;

    public InterpreterTests(InterpreterFixture fx)
    {
        _fx = fx;
    }

    private static string RepoRoot =>
        Path.GetFullPath(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "..", "..", "..", "..", ".."));

    [Fact]
    public void ScannerAndParser_ReadLiteralsListsAndVectors()
    {
        var parser = new Parser(new Scanner(new StringReader("(#t #f #\\space #\\A -42 (1 . 2) #(a b))")));
        var parsed = parser.Read();
        Assert.NotNull(parsed);
        Assert.Equal("(#t #f #\\space #\\A -42 (1 . 2) #2(a b))", parsed!.ToString());
    }

    [Fact]
    public void CorePrimitives_ArithmeticStringsVectorsAndProps()
    {
        Assert.Equal(42L, _fx.EvalLast("(+ 19 23)"));
        Assert.Equal(Scheme.Boolean.TRUE, _fx.EvalLast("(zero? 0)"));
        Assert.Equal(Scheme.Boolean.FALSE, _fx.EvalLast("(zero? 7)"));
        Assert.Equal("#\\A", _fx.EvalLast("(integer->char 65)")!.ToString());
        Assert.Equal(5L, _fx.EvalLast("(string-length \"hello\")"));
        Assert.Equal("#3(1 99 3)", _fx.EvalLast("(let ((v (vector 1 2 3))) (vector-set! v 1 99) v)")!.ToString());
    }

    [Fact]
    public void ClosuresMacrosAndMultipleValues_WorkCorrectly()
    {
        Assert.Equal("(1 2 (3 4))", _fx.EvalLast("((lambda (a b . rest) (list a b rest)) 1 2 3 4)")!.ToString());
        Assert.Equal("(x y z)", _fx.EvalLast("((lambda all all) 'x 'y 'z)")!.ToString());
        Assert.Equal(30L, _fx.EvalLast("(call-with-values (lambda () (values 10 20)) +)"));
        Assert.Equal("(2 1)", _fx.EvalLast(@"
            (let ((a 1) (b 2))
              (let-syntax ((swap! (syntax-rules () ((swap! x y) (let ((t x)) (set! x y) (set! y t))))))
                (swap! a b)
                (list a b)))")!.ToString());
    }

    [Fact]
    public void FirstClassContinuations_EscapeAndMultiShot()
    {
        Assert.Equal(107L, _fx.EvalLast("(+ 100 (call/cc (lambda (k) (+ 1 (k 7)))))"));
        Assert.Equal("(11 12 13)", _fx.EvalLast(@"
            (let ((saved #f) (acc '()))
              (let ((v (+ 10 (call/cc (lambda (k) (set! saved k) 1)))))
                (set! acc (cons v acc))
                (if (< v 13)
                    (saved (- v 9))
                    (reverse acc))))")!.ToString());
    }

    [Fact]
    public void SharedSuiteScm_PassesAllAssertions()
    {
        var sw = new StringWriter();
        var origOut = Console.Out;
        Console.SetOut(sw);
        try
        {
            _fx.Interpreter.ReadFile(Path.Combine(RepoRoot, "test", "suite.scm"));
        }
        finally
        {
            Console.SetOut(origOut);
        }

        string output = sw.ToString();
        Assert.DoesNotContain("FAIL [", output);
        Assert.Contains("Failed: 0", output);
    }

    [Fact]
    public void LegacyTestScm_ExecutesCleanly()
    {
        var sw = new StringWriter();
        var origOut = Console.Out;
        Console.SetOut(sw);
        try
        {
            _fx.Interpreter.ReadFile(Path.Combine(RepoRoot, "test", "test.scm"));
        }
        finally
        {
            Console.SetOut(origOut);
        }

        string[] lines = sw.ToString().Trim().Split('\n', StringSplitOptions.RemoveEmptyEntries);
        Assert.True(lines.Length >= 4);
        Assert.Equal("3", lines[0].Trim());
        Assert.Equal("1", lines[1].Trim());
        Assert.Equal("1", lines[2].Trim());
        Assert.StartsWith("(define make-fibo-coroutine", lines[3].Trim());
    }
}
