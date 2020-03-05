//! postkeeper milter daemon for postfix/sendmail
//!

#![warn(missing_docs)]
#![warn(unused_variables)]
#![warn(dead_code)]

mod consts;

use clap::{load_yaml, App};
use consts::*;
use daemonize::Daemonize;
use std::{fs::OpenOptions, process, thread};

fn main() {
    let yaml = load_yaml!("cli.yml");
    let app = App::from_yaml(yaml);
    let mut help = Vec::new();
    app.write_help(&mut help).expect("Unable to get app help");
    let matches = app.get_matches();

    // daemonize process if forground arg is not set
    if !matches.is_present(arg::FOREGROUND) {
        let pid_file = matches
            .value_of(arg::PIDFILE)
            .unwrap_or_else(|| "/var/run/postkeeper/postkeeper.pid");

        let mut daemonize = Daemonize::new().pid_file(pid_file);

        if let Some(log_file) = matches.value_of(arg::LOGFILE) {
            let log_file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(log_file)
                .unwrap_or_else(|_| {
                    eprintln!("could not open file {}", log_file);
                    process::exit(1);
                });
            daemonize = daemonize.stdout(log_file)
        }

        match daemonize.start() {
            Ok(_) => println!("Postkeeper exited!"),
            Err(e) => {
                eprintln!("Daemon Error, {}", e);
                process::exit(1)
            }
        }

        // TODO: implement milter here
        println!("Parking thread");
        thread::park();
    } else {
        println!("Please run as daemon!")
    }
}
