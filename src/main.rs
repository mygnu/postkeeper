//! postkeeper milter daemon for postfix/sendmail
//!

#![warn(missing_docs)]
#![warn(unused_variables)]
#![warn(dead_code)]

mod config;
mod consts;
mod error;
mod milter;
mod prelude;

#[macro_use]
extern crate log;
extern crate simple_logger;

use crate::config::Config;
use clap::{load_yaml, App};
use consts::*;
use daemonize::Daemonize;
use std::{fs::OpenOptions, process};

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let mut help = Vec::new();
    app.write_help(&mut help).expect("Unable to get app help");
    let matches = app.get_matches();

    // log level progerssion error!, warn!, info!, debug! and trace!
    let log_level = match matches.occurrences_of(arg::VERBOSE) {
        0 => log::Level::Error,
        1 => log::Level::Warn,
        2 => log::Level::Info,
        3 => log::Level::Debug,
        4 | _ => log::Level::Trace,
    };

    simple_logger::init_with_level(log_level).expect("Logger Double Initialized");

    trace!("Logger initialized with {:?}", log_level);

    let config = match Config::from_args(&matches) {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            process::exit(1);
        }
    };

    // exit with error if config is not valid
    if let Err(e) = config.validate() {
        error!("{}", e);
        process::exit(1);
    }

    // run in forground if cli arg is present otherwise
    // daemonize the process
    if matches.is_present(arg::FOREGROUND) {
        milter::run(&config);
    } else {
        let mut daemonize = Daemonize::new().pid_file(config.pid_file());

        // dorp privileges to user and/or group if present in config
        if let Some(user) = config.user() {
            daemonize = daemonize.user(user)
        }

        if let Some(group) = config.group() {
            daemonize = daemonize.group(group)
        }

        // use the log file path for daemon stdout/stderr
        let stderr = OpenOptions::new()
            .write(true)
            .create(true)
            .open(config.log_file())
            .unwrap_or_else(|_| {
                error!("Failed to open log file {:?}", config.log_file());
                process::exit(1);
            });

        let stdout = stderr.try_clone().unwrap_or_else(|_| {
            error!("Failed to clone log file {:?} handle", config.log_file());
            process::exit(1);
        });

        daemonize = daemonize.stdout(stdout).stderr(stderr);

        match daemonize.start() {
            Ok(_) => info!("Postkeeper Daemonized!"),
            Err(e) => {
                error!("Daemonize process exited with Error, {}", e);
                process::exit(1)
            }
        }
        milter::run(&config);
    }
}
