// Command Line arguments and debug file
use clap::ArgMatches;
use slog::Logger;
use sloggers::Build;
use sloggers::file::FileLoggerBuilder;
use sloggers::null::NullLoggerBuilder;
use sloggers::types::Severity;
lazy_static!{
    pub static ref MATCHES: ArgMatches<'static> =
        clap_app!(rogue_ai_2nd =>
                  (version: "0.0.1")
                  (author: "kngwyu")
                  (about: "Automatic rogue 5.4.4 player")
                  (@arg DEBUG_FILE: -D --debug +takes_value "Debug File")
                  (@arg DEBUG_LEVEL: -L --level +takes_value "Debug Level")
                  (@arg MAX_LOOP: -M --maxloop +takes_value "Max Loop number")
                  (@arg INTERVAL: -I --interval +takes_value "Draw interval")
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
            builder.level(*LEVEL);
            builder.truncate();
            builder.build()
        }
        None => NullLoggerBuilder{}.build(),
    }.ok().unwrap();
}

pub const COLUMNS: usize = 80;
pub const LINES: usize = 22;
pub const INF_DIST: i32 = (COLUMNS * LINES) as i32;
