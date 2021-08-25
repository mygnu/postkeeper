//! postkeeper milter daemon for postfix/sendmail

#![warn(missing_docs)]
#![warn(unused_variables)]
#![warn(dead_code)]

mod config;
mod consts;
mod error;
mod maps;
mod milter;
mod prelude;

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

    let config = match Config::from_args(&matches) {
        Ok(config) => config,
        Err(e) => {
            log::error!("{}", e);
            process::exit(1);
        }
    };

    simple_logger::init_with_level(config.log_level())
        .expect("Logger Double Initialized");

    // exit with error if config is not valid
    if let Err(e) = config.validate() {
        log::error!("{}", e);
        process::exit(1);
    }

    // exit early if we only want to test config
    if matches.is_present(arg::TEST_CONFIG) {
        log::info!("Config Successfully Loaded: {:#?}", &config);
        process::exit(0);
    }

    if let Err(e) = maps::load_allow_map(config.allow_map_path()) {
        log::error!("Failed to load {:?}, {}", config.allow_map_path(), e);
        process::exit(1)
    }

    if let Err(e) = maps::load_block_map(config.block_map_path()) {
        log::error!("Failed to load {:?}, {}", config.block_map_path(), e);
        process::exit(1)
    }

    // run in forground if cli arg is present otherwise
    // daemonize the process
    if !matches.is_present(arg::FOREGROUND) {
        let mut daemonize = Daemonize::new()
            .working_directory(default::RUN_DIR) // default is "/"
            .pid_file(config.pid_file_path());

        // dorp privileges to user and/or group if present in config
        if let Some(user) = config.user() {
            log::debug!("Setting Daemon user to {}", user);
            daemonize = daemonize.user(user)
        }

        if let Some(group) = config.group() {
            log::debug!("Setting Daemon group to {}", group);
            daemonize = daemonize.group(group)
        }

        // use the log file path for daemon stdout/stderr
        let err_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.log_file_path())
            .unwrap_or_else(|_| {
                log::error!(
                    "Failed to open log file {:?}",
                    config.log_file_path()
                );
                process::exit(1);
            });

        let out_file = err_file.try_clone().unwrap_or_else(|_| {
            log::error!(
                "Failed to clone log file {:?} handle",
                config.log_file_path()
            );
            process::exit(1);
        });

        daemonize = daemonize.stdout(out_file).stderr(err_file);

        match daemonize.start() {
            Ok(_) => log::info!("{} Daemonized!", NAME),
            Err(e) => {
                log::error!("Daemonize process exited with Error, {}", e);
                process::exit(1)
            }
        }
    }
    milter::run(config)
}
