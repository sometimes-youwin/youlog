/*!
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

```
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
*/

use log::LevelFilter;
use std::ffi::OsStr;

type LogFn = Box<dyn Fn(&log::Record) + Sync + Send>;

/// The default environment variable containing logging filters.
pub const DEFAULT_ENV: &str = "RUST_LOG";

/// A filter for a module.
#[derive(Clone)]
struct Filter {
    /// The name of the module to filter.
    name: String,
    /// The max logging level for the module.
    level: LevelFilter,
}

/// A logger that accepts user functions. Filters are optional, and by default all logs
/// are enabled. This allows the user functions to implement their own per-level filters.
pub struct Youlog {
    max_level: LevelFilter,
    filters: Vec<Filter>,

    raw_fn: LogFn,
    info_fn: LogFn,
    warn_fn: LogFn,
    error_fn: LogFn,
    debug_fn: LogFn,
    trace_fn: LogFn,
}

impl log::Log for Youlog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        for filter in self.filters.iter().rev() {
            if !metadata.target().starts_with(&filter.name) {
                continue;
            }

            return metadata.level() <= filter.level;
        }

        false
    }

    fn log(&self, record: &log::Record) {
        (self.raw_fn)(record);
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
    /// Create a new, unconfigured logger.
    pub fn new() -> Self {
        let empty_log = Box::new(|_record: &log::Record| {});

        Self {
            max_level: LevelFilter::Trace,
            filters: Vec::new(),

            raw_fn: empty_log.clone(),
            info_fn: empty_log.clone(),
            warn_fn: empty_log.clone(),
            error_fn: empty_log.clone(),
            debug_fn: empty_log.clone(),
            trace_fn: empty_log.clone(),
        }
    }

    /// Create a new logger configured from the [`DEFAULT_ENV`] logging variable.
    pub fn new_from_default_env() -> Self {
        Self::new_with_env(DEFAULT_ENV)
    }

    /// Create a new logger configured with the environment variable given by `var_name`.
    pub fn new_with_env<T: AsRef<OsStr>>(var_name: T) -> Self {
        let mut youlog = Self::new();

        match std::env::var(var_name) {
            Ok(v) => {
                // Discard the regex, if any
                // The unwrap should be safe but supply a default anyways
                let filters = v.split('/').next().unwrap_or_default();

                for s in filters.split(',').map(|v| v.trim()) {
                    if s.is_empty() {
                        continue;
                    }

                    let mut parts = s.split('=');
                    let (name, level) =
                        match (parts.next(), parts.next().map(|p| p.trim()), parts.next()) {
                            // level,
                            // name,
                            (Some(name), None, None) => match name.parse() {
                                Ok(level) => (None, level),
                                Err(_) => (Some(name), LevelFilter::max()),
                            },
                            // name=,
                            (Some(name), Some(""), None) => (Some(name), LevelFilter::max()),
                            // name=level
                            (Some(name), Some(level), None) => match level.parse() {
                                Ok(level) => (Some(name), level),
                                Err(_) => {
                                    eprintln!("warning: invalid logging spec '{level}', ignoring");
                                    continue;
                                }
                            },
                            _ => {
                                eprintln!("warning: invalid logging spec '{s}', ignoring");
                                continue;
                            }
                        };

                    youlog = if let Some(name) = name {
                        youlog.level(name, level)
                    } else {
                        youlog.global_level(level)
                    };
                }
            }
            Err(e) => eprintln!("{e}"),
        }

        youlog
    }

    /// Initialize and consume the logger.
    pub fn init(mut self) -> Result<(), log::SetLoggerError> {
        self.filters.sort_unstable_by(|a, b| {
            let a_len = a.name.len();
            let b_len = b.name.len();

            a_len.cmp(&b_len)
        });

        log::set_max_level(self.max_level);
        log::set_boxed_logger(Box::new(self))
    }

    /// Set the log level globally. Does not override module-specific levels.
    pub fn global_level(mut self, level: LevelFilter) -> Self {
        self.max_level = level;

        self
    }

    /// Set the log level for a specific module. Overrides the global log level.
    pub fn level(mut self, module: impl AsRef<str>, level: LevelFilter) -> Self {
        let name = module.as_ref();

        if self.filters.iter().any(|v| v.name == name) {
            eprintln!("warning: level filter for '{name}' already exists, ignoring");
            return self;
        } else {
            self.filters.push(Filter {
                name: name.to_string(),
                level,
            });
        }

        self
    }

    /// Set a logging function for a given [`LevelFilter`].
    pub fn log_fn(
        mut self,
        level: LevelFilter,
        function: impl Fn(&log::Record) + Send + Sync + 'static,
    ) -> Self {
        let function = Box::new(function);
        match level {
            LevelFilter::Off => {
                eprintln!("warning: setting a log fn for LevelFilter::Off doesn't do anything");
            }
            LevelFilter::Error => self.error_fn = function,
            LevelFilter::Warn => self.warn_fn = function,
            LevelFilter::Info => self.info_fn = function,
            LevelFilter::Debug => self.debug_fn = function,
            LevelFilter::Trace => self.trace_fn = function,
        }

        self
    }

    /// Set a logging function that is called across logging levels up to the global logging level.
    ///
    /// # NOTE
    /// This logging function is called before other logging functions.
    pub fn raw_fn(mut self, function: impl Fn(&log::Record) + Send + Sync + 'static) -> Self {
        self.raw_fn = Box::new(function);

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, Level, Log, Metadata, MetadataBuilder, Record};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    /// Helper function for reading an [AtomicUsize].
    fn count(counter: &Arc<AtomicUsize>) -> usize {
        counter.load(Ordering::Relaxed)
    }

    /// Helper function for creating a pair of [Arc<AtomicUsize>] that both
    /// point to the same [AtomicUsize].
    fn create_counter() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let counter = Arc::new(AtomicUsize::new(0));
        let clone = counter.clone();

        (counter, clone)
    }

    fn create_metadata(target: &str, level: Level) -> Metadata {
        MetadataBuilder::new().target(target).level(level).build()
    }

    // TODO individual tests must be run as separate test binaries, otherwise the logger fails to init across tests
    #[test]
    fn logging() {
        let (info_counter, closure_info_counter) = create_counter();
        let (debug_counter, closure_debug_counter) = create_counter();
        // Intentionally not incremented
        let (warn_counter, closure_warn_counter) = create_counter();
        // Intentionally not incremented
        let (trace_counter, closure_trace_counter) = create_counter();

        static mut FN_INT: AtomicUsize = AtomicUsize::new(0);

        fn raw_fn(_r: &Record) {
            unsafe { FN_INT.fetch_add(1, Ordering::Relaxed) };
        }

        Youlog::new()
            .global_level(LevelFilter::Debug)
            .log_fn(LevelFilter::Info, move |r| {
                closure_info_counter.fetch_add(1, Ordering::Relaxed);
                println!("info {}", r.args().as_str().unwrap_or_default());
            })
            .log_fn(LevelFilter::Debug, move |r| {
                closure_debug_counter.fetch_add(1, Ordering::Relaxed);
                println!("debug {}", r.args().as_str().unwrap_or_default());
            })
            .log_fn(LevelFilter::Warn, move |_r| {
                println!("warn {}", count(&closure_warn_counter));
            })
            .log_fn(LevelFilter::Trace, move |r| {
                closure_trace_counter.fetch_add(1, Ordering::Relaxed);
                println!("trace {}", r.args().as_str().unwrap_or_default());
            })
            .raw_fn(raw_fn)
            .init()
            .expect("failed to init Youlog");

        assert_eq!(count(&info_counter), 0);
        assert_eq!(count(&debug_counter), 0);
        assert_eq!(count(&warn_counter), 0);
        assert_eq!(count(&trace_counter), 0);
        unsafe { assert_eq!(FN_INT.load(Ordering::Relaxed), 0) };

        info!("blah");
        trace!("failed trace");
        // Just making sure :)
        error!("failed error");

        assert_eq!(count(&info_counter), 1);
        assert_eq!(count(&debug_counter), 0);
        assert_eq!(count(&warn_counter), 0);
        assert_eq!(count(&trace_counter), 0);
        unsafe { assert_eq!(FN_INT.load(Ordering::Relaxed), 2) };

        info!("bleh");
        debug!("wee");
        trace!("failed trace");

        assert_eq!(count(&info_counter), 2);
        assert_eq!(count(&debug_counter), 1);
        assert_eq!(count(&warn_counter), 0);
        assert_eq!(count(&trace_counter), 0);
        unsafe { assert_eq!(FN_INT.load(Ordering::Relaxed), 4) };
    }

    #[test]
    fn filter_enabled() {
        let mut youlog = Youlog::new().level("test", LevelFilter::Info);

        assert!(youlog.enabled(&create_metadata("test", Level::Info)));
        assert!(youlog.enabled(&create_metadata("test::blah", Level::Info)));
        assert!(youlog.enabled(&create_metadata("test::blah::eh", Level::Info)));
        assert!(!youlog.enabled(&create_metadata("other", Level::Info)));
        assert!(!youlog.enabled(&create_metadata("test", Level::Trace)));

        assert!(!youlog.enabled(&create_metadata("test::blah", Level::Debug)));

        youlog = youlog.level("test::blah", LevelFilter::Debug);

        assert!(youlog.enabled(&create_metadata("test::blah", Level::Debug)));
    }

    #[test]
    fn env() {
        std::env::set_var(DEFAULT_ENV, "debug,test=info,other=debug,bleh=error");
        let youlog = Youlog::new_from_default_env();

        assert!(youlog.enabled(&create_metadata("test", Level::Info)));
        assert!(youlog.enabled(&create_metadata("other", Level::Debug)));
        assert_eq!(youlog.max_level, LevelFilter::Debug);

        std::env::set_var("SPECIAL_RUST_LOG", "error,test=error");
        let youlog = Youlog::new_with_env("SPECIAL_RUST_LOG");

        assert!(youlog.enabled(&create_metadata("test", Level::Error)));
        assert_eq!(youlog.max_level, LevelFilter::Error);
    }
}
