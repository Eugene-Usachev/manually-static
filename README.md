# Manually-static: Bridging the `'static` Gap with Debug-Time Safety

This crate provides `ManuallyStatic<T>`,
a powerful wrapper that allows you to manually manage `'static` lifetimes for your data.
While it uses `unsafe` under the hood, it also uses robust debug-time checks that panic on incorrect usage.
This means you can confidently assert `'static` guarantees in your code,
knowing that misuse will be caught during development and testing.

## Why `manually-static`?
In concurrent programming with threads or asynchronous operations, data often needs to be `'static` to
be shared or moved across task boundaries.
However, sometimes you have a logical
guarantee that a reference will live for the entire program's duration,
even if you can't easily prove it to the compiler through standard means.

`manually-static` empowers you to:

- Opt-in to manual `'static` management: Take control when the compiler's strictness becomes a hurdle.

- Catch errors early: Leverage `debug_assertions` to detect use-after-free scenarios or other incorrect
  dereferencing before they become hard-to-debug runtime crashes in production.

- Simplify complex lifetime annotations: Reduce boilerplate and make your code more readable in scenarios
  where `'static` is implicitly guaranteed.

## Usage

First, add `manually-static` to your `Cargo.toml`:

```toml
[dependencies]
manually-static = "1.0.1" # Or the latest version
```

## Threading Example (Illustrating `'static` need)

```rust
use manually_static::ManuallyStatic;
use std::thread;
use std::time::Duration;

struct AppConfig {
    version: String,
}

fn main() {
    let config = ManuallyStatic::new(AppConfig {
        version: String::from("1.0.0"),
    });

    // Get a 'static reference to the config.
    // This is where ManuallyStatic shines, allowing us to pass
    // a reference that the compiler would normally complain about
    // without complex ownership transfers or Arc for simple reads.
    let config_ref = config.get_ref();

    let handle = thread::spawn(move || {
        // In this thread, we can safely access the config via the 'static reference.
        // In debug builds, if `config` (the original ManuallyStatic) was dropped
        // before this thread accessed it, it would panic.

        thread::sleep(Duration::from_millis(100)); // Simulate some work

        println!("Thread: App Version: {}", config_ref.version);
    });

    handle.join().unwrap();

    // config is dropped here after the thread has finished
}
```

## Example with allocating the data on the heap

```rust
use manually_static::ManuallyStaticPtr;
use std::sync::Mutex;
use std::array;

const N: usize = 10280;
const PAR: usize = 16;

#[allow(dead_code, reason = "It is an example.")]
struct Pool(Mutex<([Vec<u8>; N], usize)>);

fn main() {
  let pool = ManuallyStaticPtr::new(Pool(Mutex::new((array::from_fn(|_| Vec::new()), 0))));
  let mut joins = Vec::with_capacity(PAR);
  
  for _ in 0..PAR {
      #[allow(unused_variables, reason = "It is an example.")]
      let pool = pool.clone();
  
      joins.push(std::thread::spawn(move || {
          /* ... do some work ... */
      }));
  }
  
  for join in joins {
      join.join().unwrap();
  }
  
  unsafe { pool.free(); }
}
```

## ⚠️ Important Considerations

- `unsafe` under the hood: While `manually-static` provides debug-time checks, 
  the underlying mechanism involves raw pointers.
  In release builds, these checks are absent,
  and misusing `ManuallyStaticRef` after the original `ManuallyStatic` has been dropped
  will lead to undefined behavior (UB).
- Use responsibly: This crate is intended for specific scenarios where you have a strong,
  provable-by-logic guarantee about the lifetime of your data,
  but the compiler's static analysis cannot infer it.
  Avoid using it as a general workaround for lifetime errors without fully understanding the implications.

`manually-static` is your trusty companion for those tricky `'static` lifetime puzzles,
offering a powerful blend of flexibility and debug-time safety!