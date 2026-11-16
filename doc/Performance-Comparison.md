# C# vs. Rust Scheme Interpreter Performance Comparison

This document summarizes the empirical performance comparison between the original **C# (.NET)** Scheme interpreter (`csharp/`) and the **Rust** port (`rust/`), both implementing R5RS-subset Scheme with **first-class continuations (`call/cc`) via stack reconstruction** and Dybvig & Waddell's portable `syntax-case` (`psyntax.pp` v7.0) macro expander, based on *Implementing Continuations for a Virtual Machine Environment* (Anthony J. Lorelli, 2006; see [`Lorelli-Project-2006.pdf`](Lorelli-Project-2006.pdf)).

---

## 1. Executive Summary

- **Pure Recursion & Tail Loops (No Continuations)**: Once warmed up, the two implementations run at **nearly identical speeds**, with **.NET 10's JIT ~9–10% faster** (`0.91x–0.92x`) than Rust on allocation-heavy recursive loops (`tak`, `tail-loop-100k`) thanks to generational bump-pointer nursery (`Gen0`) allocation vs. per-frame reference-counted heap allocation (`Rc<RefCell<EnvFrame>>`).
- **First-Class Continuations (`call/cc`)**: On continuation-intensive workloads, **Rust is 4.9x to 23.0x faster** than C#:
  - **14.4x faster** at continuation capture (`0.85 µs` vs. `12.24 µs` per capture)
  - **17.8x faster** at continuation invocation (`0.38 µs` vs. `6.82 µs` per jump)
  - **14.9x faster** on symmetric producer/consumer coroutines
  - **23.0x faster** on `ctak` (`186.1 ms` vs. `4,271.4 ms`)
- **Continuation Overhead Ratio (`ctak / tak`)**: Turning pure Takeuchi recursion (`tak`) into continuation-passing `ctak` adds **114.8x** overhead in C# due to CLR exception unwinding, compared to only **4.58x** overhead in Rust using algebraic `Result<Value, Signal>` return values.
- **Startup, Macro Expansion, and Memory**:
  - **1.48x faster** cold startup + `psyntax.pp` bootstrap (`158.2 ms` vs. `234.9 ms`)
  - **3.07x faster** end-to-end execution of the 32-check [`test/continuations.scm`](../test/continuations.scm) suite (`340.0 ms` vs. `1,043.2 ms`)
  - **2.94x lower** peak resident memory (`35.3 MB` vs. `103.7 MB` RSS)

---

## 2. Test Environment & Methodology

### Hardware & Toolchains
| Component | Specification |
| :--- | :--- |
| **CPU** | Intel(R) Xeon(R) Platinum 8481C (`x86_64`, 2 vCPUs) |
| **OS / Kernel** | Debian GNU/Linux 12, Linux `6.12.105+` |
| **Rust Toolchain** | `rustc 1.99.1` (`cargo build --release`, LLVM `-O3`) |
| **.NET Toolchain** | `.NET SDK 10.0.404` (`dotnet build -c Release`, `net10.0` Tiered JIT) |

### Measurement Methodology
1. **Direct Binary Execution**: End-to-end process timings invoke the compiled native/apphost binaries directly (`./rust/target/release/scheme` and `./csharp/src/bin/Release/net10.0/scheme`) so `cargo run` / `dotnet run` build-check overhead is completely excluded.
2. **JIT & Cache Warmup**: Every in-process microbenchmark executes untimed warmup iterations inside the same process prior to timed iterations so .NET 10's Tiered Compilation promotes hot methods (`Interpreter.Execute`, `Closure.BindArguments`, `Environment.Lookup`, etc.) to optimized Tier-1 native code before timing begins.
3. **High-Resolution In-Process Timing**: Microbenchmarks use `(time thunk)` from [`lib/init.scm`](../lib/init.scm) backed by `_get-ticks` (`100 ns` tick resolution in both `DateTime.UtcNow.Ticks` and `SystemTime::now()`), excluding stdout `display` I/O from timed regions.
4. **Memory**: Peak Resident Set Size (RSS) is measured over the full benchmark run via `/usr/bin/time -v`.

---

## 3. Macro-Level Process, Suite, and Memory Results

| Metric | Rust (`--release`) | C# (`.NET 10 Release`) | Ratio (`C# / Rust`) |
| :--- | ---: | ---: | ---: |
| **Cold Startup + `psyntax.pp` Bootstrap** (`(+ 1 2)`, median of 7 runs) | **158.22 ms** *(min 143.52 ms)* | 234.91 ms *(min 225.13 ms)* | **Rust 1.48x faster** |
| **Full [`test/continuations.scm`](../test/continuations.scm) Suite** (32 checks + `psyntax` expansion, median of 5 runs) | **339.96 ms** *(min 339.43 ms)* | 1,043.16 ms *(min 947.82 ms)* | **Rust 3.07x faster** |
| **Full 9-Workload Benchmark Suite Wall Time** | **3.137 s** | 29.357 s | **Rust 9.36x faster** |
| **Peak Resident Set Size (RSS)** | **35.3 MB** (`36,152 KB`) | 103.7 MB (`106,188 KB`) | **Rust 2.94x less RAM** |

---

## 4. Warm In-Process Microbenchmark Results

All times below are **microseconds (µs) per iteration** after warmup:

| Workload | Description | Iters | Rust Avg (µs) | C# Avg (µs) | Rust Min (µs) | C# Min (µs) | Speedup (Avg) | Speedup (Min) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **`tail-loop-100k`** | 100,000 tail-recursive iterations (`tail-sum`) | 20 | 47,449 | **43,078** | 46,827 | **42,471** | 0.91x *(C# 1.10x)* | 0.91x *(C# 1.10x)* |
| **`tak-18-12-6`** | `(tak 18 12 6)` — 63,609 calls, **no** `call/cc` | 10 | 40,610 | **37,220** | 40,342 | **35,635** | 0.92x *(C# 1.09x)* | 0.88x *(C# 1.13x)* |
| **`exception-x100`** | 100 non-local `try`/`throw` escapes via `call/cc` | 30 | **570** | 2,818 | **552** | 2,214 | **4.94x** | **4.01x** |
| **`amb-6-queens`** | McCarthy's `amb` backtracking (all 6-Queens solutions) | 10 | **6,278** | 36,822 | **5,995** | 35,939 | **5.87x** | **5.99x** |
| **`yin-yang-200`** | 200 steps of Yin-Yang nested continuation ping-pong | 30 | **263** | 1,756 | **248** | 1,671 | **6.68x** | **6.74x** |
| **`capture-k-1000`** | `(capture-k 1000)` — 1,000 sequential `call/cc` captures | 50 | **848** | 12,235 | **813** | 10,970 | **14.43x** | **13.49x** |
| **`fibo-coroutines`** | `(get-fibo-list 2000)` — symmetric coroutines | 100 | **92** | 1,374 | **86** | 1,298 | **14.93x** | **15.09x** |
| **`invoke-k-1000`** | `(invoke-k 1000)` — 1,000 continuation re-invocations | 50 | **383** | 6,817 | **358** | 6,586 | **17.80x** | **18.40x** |
| **`ctak-18-12-6`** | `(ctak 18 12 6)` — 63,609 calls with `call/cc` per call | 5 | **186,101** | 4,271,447 | **184,717** | 4,252,443 | **22.95x** | **23.02x** |

### Derived Per-Operation Costs

| Primitive Operation | Rust | C# (.NET 10) | Rust Advantage |
| :--- | ---: | ---: | ---: |
| **Single tail-recursive closure call** (`tail-loop-100k / 100,000`) | `0.47 µs` | **`0.43 µs`** | 0.91x *(C# 1.10x)* |
| **Single non-tail recursive call** (`tak / 63,609`) | `0.64 µs` | **`0.59 µs`** | 0.92x *(C# 1.09x)* |
| **Single `call/cc` capture** (`capture-k-1000 / 1,000`) | **`0.85 µs`** | `12.24 µs` | **14.4x faster** |
| **Single continuation jump** (`invoke-k-1000 / 1,000`) | **`0.38 µs`** | `6.82 µs` | **17.8x faster** |
| **Continuation overhead ratio** (`ctak / tak` on `(18 12 6)`) | **`4.58x`** | `114.76x` | **25.1x lower relative penalty** |

---

## 5. Architectural Analysis

### 5.1 Why Pure Recursion (`tak`, `tail-loop`) Is ~9% Faster in C#
When no continuations are captured or invoked, both interpreters spend nearly all execution time allocating lexical environments, evaluating compiled AST nodes, and looking up variables:

1. **Generational Bump Allocation vs. General-Purpose Heap + `Rc` Traffic**:
   - Both interpreters heap-allocate lexical environment frames so that upward continuations and closures can safely outlive the call that created them (`new Environment(...)` in [`csharp/src/Environment.cs`](../csharp/src/Environment.cs) vs. `Rc<RefCell<EnvFrame>>` in [`rust/src/env.rs`](../rust/src/env.rs)).
   - In pure recursion (`tak` creates 63,609 short-lived frames; `tail-loop-100k` creates 100,000 short-lived frames), almost 100% of frames die immediately. .NET 10's generational GC allocates `Gen0` objects via a thread-local bump pointer in a few CPU cycles and reclaims dead `Gen0` objects without visiting them, whereas Rust allocates and frees every `EnvFrame` + `Vec` individually through the system allocator (`malloc`/`free`) and increments/decrements `Rc` reference counts on `Expr`, `Env`, and `Symbol` handles.
2. **Memory/Throughput Trade-off**:
   - C#'s bump-allocation throughput comes at the cost of **2.94x higher peak memory** (`103.7 MB` vs. `35.3 MB`), whereas Rust reclaims dead frames deterministically at scope exit with zero GC pause overhead.

### 5.2 Why Continuations (`call/cc`) Are 14x–23x Faster in Rust
Both interpreters implement **stack reconstruction** rather than CPS conversion or full stack copying, preserving direct host-stack evaluation during normal execution. However, the *mechanism* used to unwind and reconstruct frames differs fundamentally between a managed VM and Rust:

1. **CLR Exception Unwinding vs. Zero-Cost Algebraic `Result` Unwinding**:
   - **C# ([`csharp/src/Interpreter.cs`](../csharp/src/Interpreter.cs), [`csharp/src/Continuation.cs`](../csharp/src/Continuation.cs))**:
     - `CallCC.Apply` executes `throw new CaptureException(proc)`.
     - Every active evaluation frame (`CondContext`, `AssignContext`, `DefineContext`, `ArgContext`, `RibContext`, `FuncContext`, `ApplyContext`, `SeqContext`, `MultArgContext`) wraps non-tail recursive calls in `try { ... } catch (CaptureException capEx) { capEx.AddContext(...); throw; }`.
     - Invoking a continuation executes `throw new InvokeException(retVal, _cList)` back to the `Evaluate` trampoline.
     - On the CLR, throwing and rethrowing exceptions invokes heavy runtime machinery: two-pass stack walking, exception-handling (EH) metadata table lookups per frame, and exception stack-trace capture (~`12.2 µs` per capture, ~`6.8 µs` per invoke).
   - **Rust ([`rust/src/interpreter.rs`](../rust/src/interpreter.rs), [`rust/src/context.rs`](../rust/src/context.rs))**:
     - Evaluation returns `Result<Value, Signal>`, where `Signal` is a plain Rust `enum` (`Signal::Capture { proc, frames }`, `Signal::Invoke { k_frames, val }`, `Signal::Error(String)`).
     - Unwinding an active frame is an ordinary integer discriminant check in registers (`Err(Signal::Capture { proc, mut frames }) => { frames.push(...); return Err(...); }`) with **zero** DWARF/SEH unwinding, **zero** stack-walking metadata lookups, and **zero** per-frame heap node allocations (~`0.85 µs` per capture, ~`0.38 µs` per invoke).

2. **Contiguous Frame Slices (`Rc<[Context]>`) vs. Doubly-Linked `ContextList` Deep Copies**:
   - In **C#** ([`csharp/src/Context.cs`](../csharp/src/Context.cs)), a captured continuation is a doubly-linked `ContextList` of heap-allocated `Context` objects. When capturing a new continuation inside a resumed continuation (which occurs constantly in coroutines, `amb`, `yin-yang`, and `ctak`), `CaptureException.AppendContext` calls `ContextList.CopyStack()`, allocating a fresh linked list node per frame.
   - In **Rust** ([`rust/src/context.rs`](../rust/src/context.rs)), `Continuation` stores captured frames in a single contiguous `Rc<[Context]>` slice. Invoking a continuation clones only the `Rc` fat pointer (`O(1)`), and re-capturing inside a resumed continuation appends older frames via a single contiguous `Vec::extend_from_slice` copy.

3. **Compiled Typed AST (`enum Expr`) vs. S-Expression Instruction Lists**:
   - In **C#** ([`csharp/src/Compiler.cs`](../csharp/src/Compiler.cs)), compiled bytecode instructions are represented as linked `Pair` lists (`List.First(expr) == Instruction.Apply`, `List.Second(expr)`, `List.Third(expr)`), requiring pointer chasing on every AST dispatch and during macro expansion (`psyntax.pp`).
   - In **Rust** ([`rust/src/compiler.rs`](../rust/src/compiler.rs)), compiled expressions are compact typed `Expr` enums (`Expr::Constant`, `Expr::Var`, `Expr::Cond`, `Expr::Apply`, etc.), improving cache locality and contributing to the **3.07x faster** execution on macro-heavy suites like [`test/continuations.scm`](../test/continuations.scm).

---

## 6. Reproducing the Benchmarks

You can run the built-in continuation benchmark suite ([`test/callcc.scm`](../test/callcc.scm)) on both implementations using `Makefile` targets from the repository root:

```bash
# Build Release binaries for both C# and Rust
make build

# Run the classic call/cc benchmark suite on Rust
make bench-rust

# Run the classic call/cc benchmark suite on C#
make bench-csharp
```
