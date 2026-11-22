# C# vs. Rust Scheme Interpreter Performance Comparison

This document summarizes the empirical performance comparison between the original **C# (.NET)** Scheme interpreter (`csharp/`) and the **Rust** port (`rust/`), both implementing R5RS-subset Scheme with **first-class continuations (`call/cc`) via stack reconstruction** and Dybvig & Waddell's portable `syntax-case` (`psyntax.pp` v7.0) macro expander, based on *Implementing Continuations for a Virtual Machine Environment* (Anthony J. Lorelli, 2006; see [`Lorelli-Project-2006.pdf`](Lorelli-Project-2006.pdf)).

---

## 1. Executive Summary

Following the addition of **compile-time lexical addressing (`(depth, slot)`)**, **direct global value slots**, **stack-allocated small-arity evaluation**, and a **compact 16-byte `Value` enum** in the Rust interpreter, **Rust is now faster than C# (.NET 10) across every single workload**:

- **Pure Recursion & Tail Loops (No Continuations)**: **Rust is 1.74x–2.01x faster** than warmed-up .NET 10 JIT (`tail-loop-100k`: `24.8 ms` vs. `43.1 ms`; `tak-18-12-6`: `18.5 ms` vs. `37.2 ms`), reversing the earlier ~9–10% deficit caused by per-call `HashMap` environment frames.
- **First-Class Continuations (`call/cc`)**: On continuation-intensive workloads, **Rust is 10.0x to 32.2x faster** than C#:
  - **26.8x faster** at continuation capture (`0.46 µs` vs. `12.24 µs` per capture)
  - **32.2x faster** at continuation invocation (`0.21 µs` vs. `6.82 µs` per jump)
  - **29.2x faster** on symmetric producer/consumer coroutines (`47 µs` vs. `1,374 µs`)
  - **30.2x faster** on `ctak` (`141.3 ms` vs. `4,271.4 ms`)
- **Continuation Overhead Ratio (`ctak / tak`)**: Turning pure Takeuchi recursion (`tak`) into continuation-passing `ctak` adds **114.8x** overhead in C# due to CLR exception unwinding, compared to **7.65x** overhead in Rust (`15.0x` lower relative penalty) using algebraic `Result<Value, Signal>` return values.
- **Startup, Macro Expansion, and Memory**:
  - **2.38x faster** cold startup + `psyntax.pp` bootstrap (`98.8 ms` vs. `234.9 ms`)
  - **5.52x faster** end-to-end execution of the 32-check [`test/continuations.scm`](../test/continuations.scm) suite (`188.8 ms` vs. `1,043.2 ms`)
  - **4.61x lower** peak resident memory (`22.5 MB` vs. `103.7 MB` RSS)

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
| **Cold Startup + `psyntax.pp` Bootstrap** (`(+ 1 2)`, median of 7 runs) | **98.82 ms** *(min 97.18 ms)* | 234.91 ms *(min 225.13 ms)* | **Rust 2.38x faster** |
| **Full [`test/continuations.scm`](../test/continuations.scm) Suite** (32 checks + `psyntax` expansion, median of 5 runs) | **188.83 ms** *(min 187.84 ms)* | 1,043.16 ms *(min 947.82 ms)* | **Rust 5.52x faster** |
| **Full 9-Workload Benchmark Suite Wall Time** | **2.17 s** | 29.357 s | **Rust 13.53x faster** |
| **Peak Resident Set Size (RSS)** | **22.5 MB** (`23,048 KB`) | 103.7 MB (`106,188 KB`) | **Rust 4.61x less RAM** |

---

## 4. Warm In-Process Microbenchmark Results

All times below are **microseconds (µs) per iteration** after warmup:

| Workload | Description | Iters | Rust Avg (µs) | C# Avg (µs) | Rust Min (µs) | C# Min (µs) | Speedup (Avg) | Speedup (Min) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **`tail-loop-100k`** | 100,000 tail-recursive iterations (`tail-sum`) | 20 | **24,805** | 43,078 | **24,229** | 42,471 | **1.74x** | **1.75x** |
| **`tak-18-12-6`** | `(tak 18 12 6)` — 63,609 calls, **no** `call/cc` | 10 | **18,475** | 37,220 | **17,956** | 35,635 | **2.01x** | **1.98x** |
| **`exception-x100`** | 100 non-local `try`/`throw` escapes via `call/cc` | 30 | **281** | 2,818 | **268** | 2,214 | **10.03x** | **8.26x** |
| **`amb-6-queens`** | McCarthy's `amb` backtracking (all 6-Queens solutions) | 10 | **2,969** | 36,822 | **2,746** | 35,939 | **12.40x** | **13.09x** |
| **`yin-yang-200`** | 200 steps of Yin-Yang nested continuation ping-pong | 30 | **107** | 1,756 | **98** | 1,671 | **16.41x** | **17.05x** |
| **`capture-k-1000`** | `(capture-k 1000)` — 1,000 sequential `call/cc` captures | 50 | **456** | 12,235 | **414** | 10,970 | **26.83x** | **26.50x** |
| **`fibo-coroutines`** | `(get-fibo-list 2000)` — symmetric coroutines | 100 | **47** | 1,374 | **44** | 1,298 | **29.23x** | **29.50x** |
| **`invoke-k-1000`** | `(invoke-k 1000)` — 1,000 continuation re-invocations | 50 | **212** | 6,817 | **200** | 6,586 | **32.16x** | **32.93x** |
| **`ctak-18-12-6`** | `(ctak 18 12 6)` — 63,609 calls with `call/cc` per call | 5 | **141,265** | 4,271,447 | **139,586** | 4,252,443 | **30.24x** | **30.46x** |

### Derived Per-Operation Costs

| Primitive Operation | Rust | C# (.NET 10) | Rust Advantage |
| :--- | ---: | ---: | ---: |
| **Single tail-recursive closure call** (`tail-loop-100k / 100,000`) | **`0.25 µs`** | `0.43 µs` | **1.74x faster** |
| **Single non-tail recursive call** (`tak / 63,609`) | **`0.29 µs`** | `0.59 µs` | **2.01x faster** |
| **Single `call/cc` capture** (`capture-k-1000 / 1,000`) | **`0.46 µs`** | `12.24 µs` | **26.8x faster** |
| **Single continuation jump** (`invoke-k-1000 / 1,000`) | **`0.21 µs`** | `6.82 µs` | **32.2x faster** |
| **Continuation overhead ratio** (`ctak / tak` on `(18 12 6)`) | **`7.65x`** | `114.76x` | **15.0x lower relative penalty** |

---

## 5. Rust Optimization Progression (Before vs. After)

Measuring the Rust interpreter across each optimization stage on the same machine highlights how much overhead came from dynamic environment hashing and `Value` size:

| Workload | Baseline Rust (`HashMap` Env) | + Lexical Addressing & Global Slots | + Small-Arity (32B `Value`) | + Small-Arity (16B `Value`) | Total Rust Speedup |
| :--- | ---: | ---: | ---: | ---: | ---: |
| **`tail-loop-100k`** (avg µs) | 46,431 | 29,807 | 29,191 | **24,805** | **1.87x** (-46.6%) |
| **`tak-18-12-6`** (avg µs) | 38,157 | 20,928 | 21,547 | **18,475** | **2.07x** (-51.6%) |
| **`exception-x100`** (avg µs) | 541 | 315 | 342 | **281** | **1.93x** (-48.1%) |
| **`amb-6-queens`** (avg µs) | 6,724 | 3,245 | 3,352 | **2,969** | **2.26x** (-55.8%) |
| **`yin-yang-200`** (avg µs) | 225 | 123 | 128 | **107** | **2.10x** (-52.4%) |
| **`capture-k-1000`** (avg µs) | 825 | 477 | 515 | **456** | **1.81x** (-44.7%) |
| **`fibo-coroutines`** (avg µs) | 80 | 47 | 49 | **47** | **1.70x** (-41.3%) |
| **`invoke-k-1000`** (avg µs) | 367 | 247 | 230 | **212** | **1.73x** (-42.2%) |
| **`ctak-18-12-6`** (avg µs) | 177,666 | 140,349 | 145,063 | **141,265** | **1.26x** (-20.5%) |
| **Cold startup + `psyntax.pp`** (ms) | 109.35 | 102.67 | — | **98.82** | **1.11x** (-9.6%) |
| **`test/continuations.scm`** (ms) | 306.43 | 208.07 | — | **188.83** | **1.62x** (-38.4%) |
| **Peak RSS** | 35.2 MB | 26.9 MB | 29.7 MB | **22.5 MB** | **-36.0% RAM** |

---

## 6. Architectural Analysis

### 6.1 Why Rust Now Leads on Pure Recursion & Tail Loops (`1.74x–2.01x`)
In the initial 1:1 port, both interpreters allocated a hash-table lexical frame per call (`new Environment(...)` in [`csharp/src/Environment.cs`](../csharp/src/Environment.cs) vs. `Rc<RefCell<HashMap<Symbol, Value>>>` in Rust), giving .NET 10's generational bump-pointer nursery (`Gen0`) a ~9% edge on allocation-heavy recursion. Three Rust-side optimizations eliminated that bottleneck:

1. **Compile-Time Lexical Addressing (`(depth, slot)`) & Direct Global Cells**:
   - [`Compiler`](../rust/src/compiler.rs) resolves every variable reference and `set!` statically into either `Expr::LocalVar { depth, slot }` / `Expr::AssignLocal` or `Expr::GlobalVar(Symbol)` / `Expr::AssignGlobal`, compiles `letrec` directly into `Expr::PushFrame`, and elides runtime frames entirely for frameless 0-argument thunks.
   - [`EnvFrame`](../rust/src/env.rs) stores flat indexed slots with an immutable `parent: Option<Rc<EnvFrame>>`, so walking `depth` frames requires zero `RefCell::borrow` or `Rc::clone` calls, and top-level `Env` is `None` (zero refcount traffic).
   - [`Symbol`](../rust/src/symbol.rs) carries a direct `global_val: RefCell<Option<Value>>` cell, making global variable lookups (`+`, `-`, `<`, `tak`, `tail-sum`, etc.) O(1) pointer dereferences with zero hashing.
2. **Stack-Allocated Small-Arity Calls & Inline 4-Slot Frames**:
   - Calls with `0..=4` arguments evaluate directly into a stack-allocated [`SmallArgs`](../rust/src/interpreter.rs) array (`[Value; 4]`) instead of allocating a temporary `Vec<Value>`, eliminating heap allocations on native primitive calls (`+`, `-`, `=`, `<`, `car`, `cdr`, `cons`, `call/cc`) and single-value continuation jumps.
   - [`EnvFrame`](../rust/src/env.rs) stores up to 4 slots inline (`FrameSlots::Inline([Value; 4])`), collapsing small closure and `letrec` frames from two heap allocations (`Rc` + `Vec`) into one.
3. **Compact 16-Byte `Value` Layout**:
   - Boxing the two rare `Vec`-carrying variants (`Value::Values(Box<Vec<Value>>)` and `Value::Rib(Box<Vec<Value>>)`) in [`rust/src/value.rs`](../rust/src/value.rs) halved `std::mem::size_of::<Value>()` from **32 bytes to 16 bytes** (enforced at compile time).
   - Without that shrink, inline `[Value; 4]` arrays were 128 bytes and actually regressed `tak`, `amb-6-queens`, and RSS (`29.7 MB`); at 16 bytes per `Value`, `[Value; 4]` fits in a single 64-byte cache line, unlocking the full speedup and dropping peak RSS to **22.5 MB** (`4.61x` less than C#).

### 6.2 Why Continuations (`call/cc`) Are 10x–32x Faster in Rust
Both interpreters implement **stack reconstruction** rather than CPS conversion or full stack copying, preserving direct host-stack evaluation during normal execution. However, the *mechanism* used to unwind and reconstruct frames differs fundamentally between a managed VM and Rust:

1. **CLR Exception Unwinding vs. Zero-Cost Algebraic `Result` Unwinding**:
   - **C# ([`csharp/src/Interpreter.cs`](../csharp/src/Interpreter.cs), [`csharp/src/Continuation.cs`](../csharp/src/Continuation.cs))**:
     - `CallCC.Apply` executes `throw new CaptureException(proc)`.
     - Every active evaluation frame (`CondContext`, `AssignContext`, `DefineContext`, `ArgContext`, `RibContext`, `FuncContext`, `ApplyContext`, `SeqContext`, `MultArgContext`) wraps non-tail recursive calls in `try { ... } catch (CaptureException capEx) { capEx.AddContext(...); throw; }`.
     - Invoking a continuation executes `throw new InvokeException(retVal, _cList)` back to the `Evaluate` trampoline.
     - On the CLR, throwing and rethrowing exceptions invokes heavy runtime machinery: two-pass stack walking, exception-handling (EH) metadata table lookups per frame, and exception stack-trace capture (~`12.2 µs` per capture, ~`6.8 µs` per invoke).
   - **Rust ([`rust/src/interpreter.rs`](../rust/src/interpreter.rs), [`rust/src/context.rs`](../rust/src/context.rs))**:
     - Evaluation returns `Result<Value, Signal>`, where `Signal` is a plain Rust `enum` (`Signal::Capture { proc, frames }`, `Signal::Invoke { k_frames, val }`, `Signal::Error(String)`).
     - Unwinding an active frame is an ordinary integer discriminant check in registers (`Err(Signal::Capture { proc, mut frames }) => { frames.push(...); return Err(...); }`) with **zero** DWARF/SEH unwinding, **zero** stack-walking metadata lookups, and **zero** per-frame heap node allocations (~`0.46 µs` per capture, ~`0.21 µs` per invoke).

2. **Contiguous Frame Slices (`Rc<Vec<Context>>`) vs. Doubly-Linked `ContextList` Deep Copies**:
   - In **C#** ([`csharp/src/Context.cs`](../csharp/src/Context.cs)), a captured continuation is a doubly-linked `ContextList` of heap-allocated `Context` objects. When capturing a new continuation inside a resumed continuation (which occurs constantly in coroutines, `amb`, `yin-yang`, and `ctak`), `CaptureException.AppendContext` calls `ContextList.CopyStack()`, allocating a fresh linked list node per frame.
   - In **Rust** ([`rust/src/continuation.rs`](../rust/src/continuation.rs)), `Continuation` stores captured frames in a single contiguous `Rc<Vec<Context>>`. Invoking a continuation clones only the `Rc` pointer (`O(1)`), and re-capturing inside a resumed continuation appends older frames via a single contiguous `Vec::extend_from_slice` copy.

3. **Compiled Typed AST (`enum Expr`) vs. S-Expression Instruction Lists**:
   - In **C#** ([`csharp/src/Compiler.cs`](../csharp/src/Compiler.cs)), compiled bytecode instructions are represented as linked `Pair` lists (`List.First(expr) == Instruction.Apply`, `List.Second(expr)`, `List.Third(expr)`), requiring pointer chasing on every AST dispatch and during macro expansion (`psyntax.pp`).
   - In **Rust** ([`rust/src/compiler.rs`](../rust/src/compiler.rs)), compiled expressions are compact typed `Expr` enums (`Expr::Constant`, `Expr::LocalVar`, `Expr::GlobalVar`, `Expr::Cond`, `Expr::Apply`, etc.), improving cache locality and contributing to the **5.52x faster** execution on macro-heavy suites like [`test/continuations.scm`](../test/continuations.scm).

---

## 7. Reproducing the Benchmarks

You can run the built-in continuation benchmark suite ([`test/callcc.scm`](../test/callcc.scm)) on both implementations using `Makefile` targets from the repository root:

```bash
# Build Release binaries for both C# and Rust
make build

# Run the classic call/cc benchmark suite on Rust
make bench-rust

# Run the classic call/cc benchmark suite on C#
make bench-csharp
```
