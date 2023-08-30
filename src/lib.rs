/*!
A thin logging implementation for Rust's [log](https://github.com/rust-lang/log) facade.

This crate allows for providing custom functions to the logger.

Examples where this might be useful:

* Logging logic needs to be different across log levels
* Another application's logger is being used like with [godot-rust](https://github.com/godot-rust)
* An existing crate is too opinionated in how it handles logging

## Example

```
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

Log level filters are still WIP, but those can just be implemented in a logging function anyways :)

# License

MPL-2.0
*/

use log::LevelFilter;

type LogFn = Box<dyn Fn(&log::Record) + Sync + Send>;

pub struct Youlog {
    max_level: LevelFilter,

    info_fn: LogFn,
    warn_fn: LogFn,
    error_fn: LogFn,
    debug_fn: LogFn,
    trace_fn: LogFn,
}

impl log::Log for Youlog {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // TODO actually check if the module is enabled?
        true
    }

    fn log(&self, record: &log::Record) {
        match record.level() {
            log::Level::Error => (self.error_fn)(record),
            log::Level::Warn => (self.warn_fn)(record),
            log::Level::Info => (self.info_fn)(record),
            log::Level::Debug => (self.debug_fn)(record),
            log::Level::Trace => (self.trace_fn)(record),
        }
    }

    fn flush(&self) {}
}

impl Youlog {
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_max_level(self.max_level);
        log::set_boxed_logger(Box::new(self))
    }
}

pub struct Builder {
    max_level: LevelFilter,

    info_fn: LogFn,
    warn_fn: LogFn,
    error_fn: LogFn,
    debug_fn: LogFn,
    trace_fn: LogFn,
}

impl Builder {
    /// Create a new builder for `Youlog`.
    pub fn new() -> Self {
        let empty_log = Box::new(|_record: &log::Record| {});

        Self {
            max_level: LevelFilter::Trace,

            info_fn: empty_log.clone(),
            warn_fn: empty_log.clone(),
            error_fn: empty_log.clone(),
            debug_fn: empty_log.clone(),
            trace_fn: empty_log.clone(),
        }
    }

    /// Consume the builder and produce a `Youlog`.
    pub fn build(self) -> Youlog {
        Youlog {
            // TODO configure this from something else?
            max_level: LevelFilter::Trace,

            info_fn: self.info_fn,
            warn_fn: self.warn_fn,
            error_fn: self.error_fn,
            debug_fn: self.debug_fn,
            trace_fn: self.trace_fn,
        }
    }

    /// Consume the builder and immediately init the logger.
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        self.build().init()
    }

    /// Set the log level globally. Does not override module-specific levels.
    pub fn global_level(mut self, level: LevelFilter) -> Self {
        self.max_level = level;

        self
    }

    /// Set the log level for a specific module. Overrides the global log level.
    #[allow(dead_code)]
    fn level<T: AsRef<str>>(self, _module: T, _level: LevelFilter) -> Self {
        todo!("not yet implemented")
    }

    /// Set a logging function for a given [`LevelFilter`].
    ///
    /// # Note
    /// Setting a logging function for [`LevelFilter::Off`] doesn't do anything but also _won't_ panic.
    pub fn log_fn(
        mut self,
        level: LevelFilter,
        function: impl Fn(&log::Record) + Send + Sync + 'static,
    ) -> Self {
        let function = Box::new(function);
        match level {
            LevelFilter::Off => {
                #[cfg(debug_assertions)]
                eprintln!("setting a log fn for LevelFilter::Off doesn't do anything");
            }
            LevelFilter::Error => self.error_fn = function,
            LevelFilter::Warn => self.warn_fn = function,
            LevelFilter::Info => self.info_fn = function,
            LevelFilter::Debug => self.debug_fn = function,
            LevelFilter::Trace => self.trace_fn = function,
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        let counter = Arc::new(AtomicUsize::new(0));
        let closure_counter = counter.clone();

        let b =
            Builder::new()
                .global_level(LevelFilter::Info)
                .log_fn(LevelFilter::Info, move |r| {
                    closure_counter.fetch_add(1, Ordering::Relaxed);
                    println!("info {}", r.args().as_str().unwrap_or_default());
                });
        let youlog = b.build();

        youlog.init().expect("failed to start logger");

        log::info!("wee");
        log::info!("blah");

        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }
}
