// Command Line arguments and debug file
use clap::ArgMatches;
use slog::Logger;
use sloggers::types::Severity;
use sloggers::Build;
use sloggers::file::FileLoggerBuilder;
use sloggers::null::NullLoggerBuilder;
lazy_static!{
    pub static ref MATCHES: ArgMatches<'static> =
        clap_app!(rogue_ai_2nd =>
                  (version: "0.0.1")
                  (author: "kngwyu")
                  (about: "Automatic rogue 5.4.4 player")
                  (@arg DEBUG_FILE: -D --debug +takes_value "debug file")
                  (@arg DEBUG_LEVEL: -L --level +takes_value "debug level")
                  (@arg ITER: -I --iter +takes_value "Play Times")
                  (@arg VIS: -V --vis "Visualize")
        )
        .get_matches();
    pub static ref LEVEL: Severity = match MATCHES.value_of("DEBUG_LEVEL") {
        Some(s) => match s {
            "1" | "Critical" | "critical" => Severity::Critical,
            "2" | "Error" | "error" => Severity::Error,
            "3" | "Warning" | "warning" => Severity::Warning,
            "4" | "Info" | "info" => Severity::Info,
            "5" | "Debug" | "debug" => Severity::Debug,
            "6" | "Trace" | "trace" => Severity::Trace,
            _ => Severity::Warning,
        }
        None => Severity::Warning,
    };
    pub static ref LOGGER: Logger = match MATCHES.value_of("DEBUG_FILE") {
        Some(s) => {
            let mut builder = FileLoggerBuilder::new(s);
            builder.level(Severity::Debug);
            builder.truncate();
            builder.build()
        }
        None => NullLoggerBuilder{}.build(),
    }.ok().unwrap();
}

pub const COLUMNS: usize = 80;
pub const LINES: usize = 22;
