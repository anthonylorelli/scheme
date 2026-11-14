DOTNET ?= $(shell command -v dotnet 2>/dev/null || echo $(HOME)/.dotnet/dotnet)
CARGO  ?= cargo

.PHONY: all build build-csharp build-rust test test-csharp test-rust run-csharp run-rust bench-rust bench-csharp clean

all: build

build: build-csharp build-rust

build-csharp:
	$(DOTNET) build csharp/Scheme.sln -c Release

build-rust:
	$(CARGO) build --manifest-path rust/Cargo.toml --release

test: test-csharp test-rust

test-csharp:
	$(DOTNET) test csharp/Scheme.sln -c Release

test-rust:
	$(CARGO) test --manifest-path rust/Cargo.toml --release

run-csharp:
	$(DOTNET) run --project csharp/src/Scheme.csproj -c Release

run-rust:
	$(CARGO) run --manifest-path rust/Cargo.toml --release

bench-rust: build-rust
	./rust/target/release/scheme test/callcc.scm

bench-csharp: build-csharp
	$(DOTNET) run --project csharp/src/Scheme.csproj -c Release --no-build -- test/callcc.scm

clean:
	$(DOTNET) clean csharp/Scheme.sln
	$(CARGO) clean --manifest-path rust/Cargo.toml
