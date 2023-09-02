# youlog

A thin logging implementation for Rust's [log](https://github.com/rust-lang/log) facade.

This crate allows for providing custom functions to the logger.

Examples where this might be useful:

- Logging logic needs to be different across log levels
- Another application's logger is being used like with [godot-rust](https://github.com/godot-rust)
- An existing crate is too opinionated in how it handles logging

## Features

- Setting logging functions per log level
- Setting a logging function across all log levels
- Filtering logs per module/filter
- Initializing filters from an environment variable (`RUST_LOG` by default)

## Example

```rs
use log::LevelFilter;
use youlog::Youlog;

Youlog::new()
    .global_level(LevelFilter::Info)
    .log_fn(LevelFilter::Info, |record| {
        println!("info {}", record.args().as_str().unwrap_or_default());
    })
    .raw_fn(|record| {
        println!("raw {}", record.args().as_str().unwrap_or_default());
    })
    .level("some_module", LevelFilter::Error)
    .init()
    .expect("unable to init logger");

log::info!("this is an info log!");
```

# License

MPL-2.0

Filter implementation referenced from [`env_logger`](https://github.com/rust-cli/env_logger).
