use crate::entity::options::Options;
use log::LevelFilter;
use std::process;
use syslog::{BasicLogger, Facility, Formatter3164};

pub(crate) fn setup_logging(options: &Options) -> anyhow::Result<(), anyhow::Error> {
    // try to setup unix sys logging first
    let syslog_formatter = Formatter3164 {
        hostname: None,
        pid: process::id(),
        facility: Facility::LOG_USER,
        process: "easydep".to_string(),
    };
    match syslog::unix(syslog_formatter) {
        Ok(logger) => {
            // unix logging possible, use that
            let basic_logger = BasicLogger::new(logger);
            log::set_boxed_logger(Box::new(basic_logger))?;
        }
        Err(_) => {
            // unix logging not possible, use console logging instead
            simple_logger::init()?;
        }
    }

    // set the default logging level
    if options.debug {
        log::set_max_level(LevelFilter::Trace);
    } else {
        log::set_max_level(LevelFilter::Info);
    }
    Ok(())
}
