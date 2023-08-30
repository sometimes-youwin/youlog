# youlog

A thin logging implementation for Rust's [log](https://github.com/rust-lang/log) facade.

This crate allows for providing custom functions to the logger.

Examples where this might be useful:

* Logging logic needs to be different across log levels
* Another application's logger is being used like with [godot-rust](https://github.com/godot-rust)
* An existing crate is too opinionated in how it handles logging

## Example

```rs
use log::LevelFilter;
use youlog::Builder;

Builder::new()
    .global_level(LevelFilter::Info)
    .log_fn(LevelFilter::Info, |r| {
        println!("info {}", r.args().as_str().unwrap_or_default());
    })
    .init()
    .expect("unable to init logger");

log::info!("this is an info log!");
```

## Status

Module level filters are still WIP, but those can just be implemented in a logging function anyways :)

# License

MPL-2.0
