

<h1>
<picture>
<source srcset="https://github.com/facet-rs/facet/raw/main/static/logo-v2/logo-only.webp">
<img src="https://github.com/facet-rs/facet/raw/main/static/logo-v2/logo-only.png" height="35" alt="Facet logo - a reflection library for Rust">
</picture> &nbsp; facet
</h1>

[![experimental](https://img.shields.io/badge/status-experimental-yellow)](https://github.com/fasterthanlime/facet)
[![free of syn](https://img.shields.io/badge/free%20of-syn-hotpink)](https://github.com/fasterthanlime/free-of-syn)
[![crates.io](https://img.shields.io/crates/v/facet.svg)](https://crates.io/crates/facet)
[![documentation](https://docs.rs/facet/badge.svg)](https://docs.rs/facet)
[![MIT/Apache-2.0 licensed](https://img.shields.io/crates/l/facet.svg)](./LICENSE)

Thanks to all individual and corporate sponsors, without whom this work could not exist:

<p> <a href="https://ko-fi.com/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/ko-fi-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/ko-fi-light.svg" height="40" alt="Ko-fi">
    </picture>
</a> <a href="https://github.com/sponsors/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/github-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/github-light.svg" height="40" alt="GitHub Sponsors">
    </picture>
</a> <a href="https://patreon.com/fasterthanlime">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/patreon-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/patreon-light.svg" height="40" alt="Patreon">
    </picture>
</a> <a href="https://zed.dev">
    <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/zed-dark.svg">
    <img src="https://github.com/facet-rs/facet/raw/main/static/sponsors-v2/zed-light.svg" height="40" alt="Zed">
    </picture>
</a> </p>
             

facet provides "const fn" reflection for Rust.

The `Facet` trait is meant to be derived for _every single type in the Rust
ecosystem_, and can be used to replace many other derive macros.

```rust,ignore
pub unsafe trait Facet: Sized {
    const SHAPE: &'static Shape;
    // (other fields ignored)
}
```

Whereas crates like `serde` derive _code_ using the heavy `syn`, `facet` derives
data with the light and fast `unsynn`.

That data does not make compile times balloon due to heavy monomorphization. It
can be used to reason about types at runtime — which even allows doing
specialization.

The `SHAPE` associated constant fully describes a type:

  * Whether it's a struct, an enum, or a scalar
  * All fields, variants, offsets, discriminants, memory layouts
  * VTable for various standard traits:
    * Display, Debug, Clone, Default, Drop etc.

## Use case: inspection, pretty printing, debugging, specialization

The `Debug` trait is severely limited because it cannot be specialized.

`facet-pretty` provides pretty printing of any type that implements `Facet`:

```rust,ignore
    let address = Address {
        street: "123 Main St".to_string(),
        city: "Wonderland".to_string(),
        country: "Imagination".to_string(),
    };

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        address,
    };

    println!("Default pretty-printing:");
    println!("{}", person.pretty());
```

```bash
facet on  main [!] via 🦀 v1.86.0
❯ cargo run --example basic_usage
   Compiling facet-pretty v0.1.2 (/Users/amos/bearcove/facet/facet-pretty)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running `target/debug/examples/basic_usage`
Default pretty-printing:
Person {
  name: Alice,
  age: 30,
  address: Address {
    street: 123 Main St,
    city: Wonderland,
    country: Imagination,
  },
}
```

(Note: the default pretty-printing shows ANSI colors).

Facet knows the type inside the `T`, so it's able to format it:

```rust,ignore
use facet_pretty::FacetPretty;

#[derive(Debug, Facet)]
struct Person {
    name: String,
}

fn main() {
    let alice = Person {
        name: "Alice".to_string(),
    };
    let bob = Person {
        name: "Bob".to_string(),
    };
    let carol = Person {
        name: "Carol".to_string(),
    };

    println!("{}", vec![alice, bob, carol].pretty());
}
```

```bash
facet on  main [!?] via 🦀 v1.86.0
❯ cargo run --example various_vecs
   Compiling facet-pretty v0.1.2 (/Users/amos/bearcove/facet/facet-pretty)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running `target/debug/examples/various_vecs`
Vec<Person> [
  Person {
    name: Alice,
  },
  Person {
    name: Bob,
  },
  Person {
    name: Carol,
  },
]
```

Because we know the shape of `T`, we can format different things differently,
if we wanted to:

```rust,ignore
    let mut file = std::fs::File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut bytes = vec![0u8; 128];
    std::io::Read::read_exact(&mut file, &mut bytes).expect("Failed to read from /dev/urandom");
    println!("{}", bytes.pretty());
```

```bash
facet on  main [!] via 🦀 v1.86.0
❯ cargo run --example vec_u8
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/examples/vec_u8`
Vec<u8>
  aa c5 ce 2a 79 95 a6 c6 63 ca 69 5f 12 d5 7e fc
  f4 40 60 48 c4 ee 10 7c 12 a2 67 3d 2f 9a c4 ca
  b3 7e 91 5c 67 16 41 35 92 31 22 0f 23 6a ad c1
  f4 b3 c2 60 38 13 02 47 25 7e f9 48 9b 11 b5 0e
  cb 5d c6 b1 43 23 bd a7 8c 6c 7d e6 7b 72 b7 26
  1a 2c e2 b8 e9 1a a6 e7 f6 b2 9b c7 88 76 d2 be
  59 79 27 00 0b 3e 88 a3 ce 8a 14 ec 72 f9 eb 23
  d4 36 93 a5 e9 b9 00 de 6a 3f 64 b8 49 05 3f 22
```

And because we can make this decision at runtime, it can be an option on the pretty-printer itself:

```rust,ignore
/// A formatter for pretty-printing Facet types
pub struct PrettyPrinter {
    indent_size: usize,
    max_depth: Option<usize>,
    color_generator: ColorGenerator,
    use_colors: bool,
    // ⬇️ here
    list_u8_as_bytes: bool,
}
```

This is just a pretty printer, but an imaginative mind could come up with...

  * A fully inspectable program state, through a browser interface?
  * A modern debugger, exposing all the standard traits and then some instead of a bag of pointers?

## Use case: (de)serialization

The `facet-peek` and `facet-poke` allow reading and writing (constructing,
initializing) any type that implements `Facet` — this makes it trivial to
write deserializers, see `facet-json`, `facet-yaml`, `facet-urlencoded`, etc.

Say we have this struct:

```rust,ignore
use facet::Facet;

#[derive(Debug, PartialEq, Eq, Facet)]
struct FooBar {
    foo: u64,
    bar: String,
}
```

We can build it fully through reflection:

```rust,ignore
// outer code: we know the type of `FooBar` — we pass `poke`
let (poke, guard) = Poke::alloc::<FooBar>();

{
    // inner code: all we have is a `poke` — our function is not generic,
    // `Poke` is not generic.
    let mut poke = poke.into_struct();
    poke.set_by_name("foo", OpaqueConst::new(&42u64))
        .unwrap();

    {
        let bar = String::from("Hello, World!");
        poke.set_by_name("bar", OpaqueConst::new(&bar))
            .unwrap();
        // bar has been moved out of
        core::mem::forget(bar);
    }
}

// outer code: we know the type of `FooBar` again, we can
// move out of the `Poke`
let foo_bar = poke.build::<FooBar>(Some(guard));
```

The `inner code` here is the kind of code you would write in a deserializer, for example.

## Use cases: parsing CLI arguments

Facet allows arbitrary attributes (WIP) so you can use it for specifying whether a CLI
argument should be positional or named, for example:

```rust,ignore
use facet::Facet;

#[derive(Facet)]
struct Args {
    #[facet(positional)]
    path: String,

    #[facet(named, short = 'v')]
    verbose: bool,

    #[facet(named, short = 'j')]
    concurrency: usize,
}

let args: Args = facet_args::from_slice(&["--verbose", "--concurrency", "14", "example.rs"]);
eprintln!("args: {}", args.pretty());
```

```bash
facet on  args [!] via 🦀 v1.86.0
❯ RUST_LOG=info nt run --no-capture test_arg_parse
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
────────────
 Nextest run ID a8ab7183-8333-465f-a94f-6036576f19c2 with nextest profile: default
    Starting 1 test across 30 binaries (66 tests skipped)
       START             facet-args::simple test_arg_parse
[INFO  simple] Logging and color backtrace initialized

running 1 test
args: Args {
  path: example.rs,
  verbose: true,
  concurrency: 14,
}
test test_arg_parse ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

        PASS [   0.005s] facet-args::simple test_arg_parse
────────────
     Summary [   0.005s] 1 test run: 1 passed, 66 skipped
```

## Use cases: beyond

This could be extended to allow RPC, there could be an analoguous derive for traits,
it could export statics so that binaries may be inspected — shapes would then be available
instead of / in conjunction with debug info.

HTTP routing is a form of deserialization.

This is suitable for all the things serde is bad at: binary formats (specialize
for Vec<u8> without a serde_bytes hack), it could be extended to support formats
like KDL/XML.

I want the derive macros to support arbitrary attributes eventually, which will also
be exposed through `Shape`.

The types are all `non_exhaustive`, so there shouldn't be churn in the
ecosystem: crates can do graceful degradation if some types don't implement the
interfaces they expect.

If you have questions or ideas, please open a GitHub issue or discussion — I'm
so excited about this.

## Ecosystem

The core crates, `facet-trait`, `facet-types` etc. are nostd-friendly.

The main `facet` crate re-exports symbols from:

- [facet-core](https://github.com/facet-rs/facet/tree/main/facet-core), which defines the main components:
  - The `Facet` trait and implementations for foreign types (mostly `libstd`)
  - The `Shape` struct along with various vtables and the whole `Def` tree
  - Type-erased pointer helpers like `OpaqueUninit`, `OpaqueConst`, and `Opaque`
  - Autoderef specialization trick needed for `facet-derive`
- [facet-derive](https://github.com/facet-rs/facet/tree/main/facet-derive), which implements the `Facet` derive attribute as a fast/light proc macro powered by [unsynn](https://docs.rs/unsynn)

For struct manipulation, the following are available:

- [facet-peek](https://github.com/facet-rs/facet/tree/main/facet-peek), which allows reading arbitrary `Facet` types
- [facet-poke](https://github.com/facet-rs/facet/tree/main/facet-poke), which allows building/altering arbitrary `Facet` types

facet supports deserialization from multiple data formats through dedicated crates:

- [facet-json](https://github.com/facet-rs/facet/tree/main/facet-json): JSON deserialization
- [facet-yaml](https://github.com/facet-rs/facet/tree/main/facet-yaml): YAML deserialization
- [facet-msgpack](https://github.com/facet-rs/facet/tree/main/facet-msgpack): MessagePack deserialization
- [facet-urlencoded](https://github.com/facet-rs/facet/tree/main/facet-urlencoded): URL-encoded form data deserialization
- [facet-args](https://github.com/facet-rs/facet/tree/main/facet-args): CLI arguments (a-la clap)

Additionally:

- [facet-pretty](https://github.com/facet-rs/facet/tree/main/facet-pretty) is able to pretty-print Facet types.
- [facet-codegen](https://github.com/facet-rs/facet/tree/main/facet-codegen) is internal and generates some of the code of `facet-core`


## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/facet-rs/facet/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/facet-rs/facet/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.