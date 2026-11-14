# Scheme Interpreter (C# & Rust)

An R5RS-subset Scheme interpreter featuring **first-class continuations** (`call/cc`) implemented via **stack reconstruction** and hygienic/unhygienic macros powered by Dybvig & Waddell's portable `syntax-case` (`psyntax.pp` v7.0), based on *Implementing Continuations for a Virtual Machine Environment* (Anthony J. Lorelli, 2006; see [`doc/Lorelli-Project-2006.pdf`](doc/Lorelli-Project-2006.pdf)).

The repository contains both the original **C# (.NET)** implementation and a complete **Rust** port sharing the runtime initialization library and Scheme test/benchmark suite.

## Repository Structure

```text
.
├── Makefile          # Unified build, test, REPL, and benchmark targets
├── csharp/           # C# (.NET) implementation and xUnit test suite
│   ├── Scheme.sln
│   ├── src/          # Original C# interpreter sources + SDK-style Scheme.csproj
│   └── tests/        # xUnit integration & unit tests (Scheme.Tests.csproj)
├── rust/             # Rust implementation and Cargo test suite
│   ├── Cargo.toml
│   ├── src/          # Rust interpreter library + CLI/REPL binary
│   └── tests/        # Rust integration tests
├── lib/              # Shared Scheme runtime initialization files
│   ├── psyntax.pp    # Portable syntax-case v7.0 macro expander
│   └── init.scm      # Runtime helpers, benchmark harness, and define-macro
├── test/             # Shared Scheme tests and continuation benchmarks
│   ├── suite.scm     # End-to-end R5RS, macro, and continuation test suite
│   ├── test.scm      # Macro expansion & evaluation smoke tests
│   ├── coroutines.scm
│   ├── exceptions.scm
│   ├── benchmark.scm
│   └── callcc.scm    # Continuation benchmark suite (tak, ctak, coroutines, etc.)
└── doc/
    └── Lorelli-Project-2006.pdf
```

## Building and Testing

### Using `make` (Both Versions)

```bash
# Build both C# and Rust interpreters
make build

# Run automated test suites for both C# and Rust
make test

# Build or test individually
make build-csharp
make test-csharp
make build-rust
make test-rust
```

### Rust (`cargo`)

```bash
cargo build --manifest-path rust/Cargo.toml --release
cargo test --manifest-path rust/Cargo.toml --release

# Start interactive REPL
./rust/target/release/scheme

# Execute a Scheme file
./rust/target/release/scheme test/suite.scm

# Load a Scheme file then drop into the REPL
./rust/target/release/scheme -f test/benchmark.scm
```

### C# (`dotnet`)

```bash
dotnet build csharp/Scheme.sln -c Release
dotnet test csharp/Scheme.sln -c Release

# Start interactive REPL
dotnet run --project csharp/src/Scheme.csproj -c Release

# Execute a Scheme file
dotnet run --project csharp/src/Scheme.csproj -c Release -- test/suite.scm

# Load a Scheme file then drop into the REPL
dotnet run --project csharp/src/Scheme.csproj -c Release -- -f test/benchmark.scm
```
