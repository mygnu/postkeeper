//! Postkeeper milter and daemon configuration

use crate::consts::{arg, default};
use crate::prelude::*;
use clap::ArgMatches;
use ini::Ini;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::time::Duration;

// holds config as a global state
// later used by milter callback functions
static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn init_global_conf(config: Config) {
    CONFIG
        .set(config)
        .expect("configuration already initialized")
}

pub fn global_conf() -> &'static Config {
    CONFIG.get().expect("configuration not initialized")
}

/// holds Configuration for milter and daemon
#[derive(Debug)]
pub struct Config {
    pid_file: PathBuf,
    log_file: PathBuf,
    log_level: log::Level,
    on_block_action: milter::Status,
    reload_interval: Duration,
    allow_map: PathBuf,
    block_map: PathBuf,
    socket: String,
    user: Option<String>,
    group: Option<String>,
}

impl Config {
    /// Generate Config from CLI arguments
    /// reads from default config file or custom path if provided in args
    /// replaces values from args if provided
    /// cli args have the highest precedent
    pub fn from_args(matches: &ArgMatches) -> Result<Self> {
        let mut conf: Config;

        if let Some(conf_path) = matches.value_of(arg::CONF) {
            conf = Config::from_conf_file(conf_path)?;
        } else {
            conf = Config::from_conf_file(default::CONFIG_PATH)?;
        }

        if let Some(allow_map) = matches.value_of(arg::ALLOW_MAP) {
            conf.allow_map = PathBuf::from(allow_map)
        }

        if let Some(block_map) = matches.value_of(arg::BLOCK_MAP) {
            conf.block_map = PathBuf::from(block_map)
        }

        if let Some(pid_file) = matches.value_of(arg::PIDFILE) {
            conf.pid_file = PathBuf::from(pid_file);
        }

        if let Some(log_file) = matches.value_of(arg::LOGFILE) {
            conf.log_file = PathBuf::from(log_file);
        }

        if let Some(socket) = matches.value_of(arg::SOCKET).map(String::from) {
            conf.socket = socket;
        }

        if let Some(user) = matches.value_of(arg::USER).map(String::from) {
            conf.user = Some(user);
        }

        if let Some(group) = matches.value_of(arg::GROUP).map(String::from) {
            conf.group = Some(group);
        }

        if let Some(on_block_action) = matches
            .value_of(arg::ON_BLOCK_ACTION)
            .map(|on_block_action| match on_block_action {
                "discard" => milter::Status::Discard,
                "continue" => milter::Status::Continue,
                _ => milter::Status::Reject,
            })
        {
            conf.on_block_action = on_block_action;
        }

        if matches.is_present(arg::VERBOSE) {
            // log level progerssion error!, warn!, info!, debug! and trace!
            conf.log_level = match matches.occurrences_of(arg::VERBOSE) {
                0 => log::Level::Error,
                1 => log::Level::Warn,
                2 => log::Level::Info,
                3 => log::Level::Debug,
                _ => log::Level::Trace,
            };
        }

        Ok(conf)
    }

    /// Validate configuration, daemon and milter should be able to run if
    /// validation returns without an error
    pub fn validate(&self) -> Result<()> {
        use Validation::*;
        // assume valid state before performing checks
        let mut state = Valid;

        if !is_socket_valid(self.socket()) {
            log::error!("{:?} socket cannot be used", self.socket());
            state = Invalid
        }

        if file_permissions(self.allow_map_path()).is_err() {
            log::error!("{:?} is not a valid file", self.allow_map_path());
            state = Invalid
        }

        if file_permissions(self.block_map_path()).is_err() {
            log::error!("{:?} is not a valid file", self.block_map_path());
            state = Invalid
        }

        if state == Invalid {
            Err(Error::config_err("Invalid Configuration!"))
        } else {
            log::trace!("Config validation success!");
            Ok(())
        }
    }

    pub fn allow_map_path(&self) -> &PathBuf {
        &self.allow_map
    }

    pub fn block_map_path(&self) -> &PathBuf {
        &self.block_map
    }

    pub fn pid_file_path(&self) -> &PathBuf {
        &self.pid_file
    }

    pub fn log_file_path(&self) -> &PathBuf {
        &self.log_file
    }

    pub fn socket(&self) -> &str {
        &self.socket
    }

    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    pub fn log_level(&self) -> log::Level {
        self.log_level
    }

    pub fn on_block_action(&self) -> milter::Status {
        self.on_block_action
    }

    pub fn reload_interval(&self) -> Duration {
        self.reload_interval
    }

    /// Builds config from config ini path
    /// uses default values if not defined in the config
    /// to allow only define variable that require a change
    /// NOTE:
    /// Errors if cannot read config file
    fn from_conf_file(path: impl AsRef<Path>) -> Result<Self> {
        println!("loading config from path {:?}", path.as_ref());

        let ini = Ini::load_from_file(path)?;
        let section = ini.general_section();

        let allow_map = section
            .get("allow_map")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default::ALLOW_MAP));

        let block_map = section
            .get("block_map")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default::BLOCK_MAP));

        let pid_file = section
            .get("pid_file")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default::PIDFILE));

        let log_file = section
            .get("log_file")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default::LOGFILE));

        let socket = section
            .get("socket")
            .map(String::from)
            .unwrap_or_else(|| String::from(default::SOCKET));

        let user = section.get("user").map(String::from);
        let group = section.get("group").map(String::from);

        let log_level = section
            .get("log_level")
            .map(|level| {
                // log level progression log::error!, log::warn!, log::info!,
                // log::debug! and log::trace! default to error
                match level {
                    "warn" => log::Level::Warn,
                    "info" => log::Level::Info,
                    "debug" => log::Level::Debug,
                    "trace" => log::Level::Trace,
                    _ => log::Level::Error,
                }
            })
            .unwrap_or_else(|| log::Level::Error);

        let on_block_action = section
            .get("on_block_action")
            .map(|on_block_action| match on_block_action {
                "discard" => milter::Status::Discard,
                "continue" => milter::Status::Continue,
                _ => default::MILTER_STATUS,
            })
            .unwrap_or_else(|| default::MILTER_STATUS);

        let reload_interval = section
            .get("reload_interval")
            .unwrap_or(default::RELOAD_INTERVAL)
            .parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|e| {
                let msg = format!("Error parsing reload_interval {:?}", e);
                Error::config_err(msg)
            })?;

        Ok(Self {
            allow_map,
            block_map,
            pid_file,
            log_file,
            socket,
            user,
            group,
            log_level,
            on_block_action,
            reload_interval,
        })
    }
}

// Holds config validation state
#[derive(PartialEq)]
enum Validation {
    Valid,
    Invalid,
}

// Validates socket address for format and connectivity
// milter would fail if socket is already in use
fn is_socket_valid(socket: &str) -> bool {
    use std::net::TcpListener;
    log::trace!("validating socket {}", socket);
    // example socket `inet:11210@127.0.0.1`
    let parts: Vec<&str> = socket.split(':').collect();

    // socket must start with `inet`
    if parts.get(0) != Some(&"inet") {
        log::trace!("socket must start with `inet`");
        return false;
    }
    let addr = parts.get(1);
    if addr.is_none() {
        return false;
    }
    let addr: Vec<&str> = addr.unwrap_or(&"").split('@').collect();

    let tcp_addr = format!(
        "{}:{}",
        addr.get(1).unwrap_or(&""),
        addr.get(0).unwrap_or(&"")
    );

    log::trace!("Checking connectivity to `{}`", tcp_addr);

    // try and check socket connectivity
    if TcpListener::bind(&tcp_addr).is_ok() {
        true
    } else {
        log::debug!("Can not connect to {}", tcp_addr);
        false
    }
}

#[derive(PartialEq)]
enum FilePermission {
    ReadOnly,
    Writable,
}

// Check if path can be written to
// returns file permission enum indicating ReadOnly or Writable
// error if file does not exist or user doesn't have permission to read it
fn file_permissions(path: impl AsRef<Path>) -> Result<FilePermission> {
    match std::fs::metadata(path.as_ref()) {
        Ok(metadata) => {
            let permission = if metadata.permissions().readonly() {
                FilePermission::ReadOnly
            } else {
                FilePermission::Writable
            };
            Ok(permission)
        }
        Err(e) => {
            log::debug!(
                "could not get metadata for {:?}, error {:?}",
                path.as_ref(),
                e
            );
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::Once;

    static INIT_LOGGING: Once = Once::new();

    fn init_logging() {
        INIT_LOGGING.call_once(|| {
            simple_logger::init_with_env().unwrap();
        });
    }

    #[test]
    fn default_config_must_load() {
        init_logging();
        // default postkeeper must load
        let config = Config::from_conf_file("assets/etc/postkeeper.ini")
            .expect("Default postkeeper.ini should load");

        assert_eq!(config.pid_file_path(), &PathBuf::from(default::PIDFILE));
        assert_eq!(config.log_file_path(), &PathBuf::from(default::LOGFILE));

        assert_eq!(config.socket(), default::SOCKET);

        assert_eq!(config.allow_map_path(), &PathBuf::from(default::ALLOW_MAP));

        assert_eq!(config.on_block_action(), milter::Status::Reject);
        assert_eq!(config.log_level(), log::Level::Error);

        assert_eq!(config.block_map_path(), &PathBuf::from(default::BLOCK_MAP));

        assert_eq!(config.user(), Some("postkeeper"));
        assert_eq!(config.group(), Some("postkeeper"));
    }

    #[test]
    fn non_existant_config() {
        init_logging();
        let err = Config::from_conf_file("non-existant.ini")
            .expect_err("Config file should not load");
        assert_eq!(err, Error::config_err("File not found"))
    }

    #[test]
    fn custom_config_values() {
        init_logging();
        let config = Config::from_conf_file("tests/conf.d/valid.ini")
            .expect("Custom ini should load");

        assert_eq!(
            config.pid_file_path(),
            &PathBuf::from("tests/sandbox/postkeeper.pid")
        );
        assert_eq!(
            config.log_file_path(),
            &PathBuf::from("tests/sandbox/postkeeper.log")
        );

        assert_eq!(config.socket(), "inet:11210@127.0.0.1");

        assert_eq!(
            config.allow_map_path(),
            &PathBuf::from("tests/sandbox/allow.map")
        );

        assert_eq!(
            config.block_map_path(),
            &PathBuf::from("tests/sandbox/block.map")
        );

        assert_eq!(config.user(), Some("user"));
        assert_eq!(config.group(), Some("group"));
        assert_eq!(config.log_level(), log::Level::Trace);
        assert_eq!(config.on_block_action(), milter::Status::Discard);

        assert_eq!(config.validate(), Ok(()))
    }

    #[test]
    fn custom_config_invalid_allow_map() {
        init_logging();
        let config =
            Config::from_conf_file("tests/conf.d/invalid-allow-map.ini")
                .expect("Custom ini should load");

        assert_eq!(
            config.allow_map_path(),
            &PathBuf::from("tests/sandbox/allow-non-existant.map")
        );

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn custom_config_invalid_block_map() {
        init_logging();
        let config =
            Config::from_conf_file("tests/conf.d/invalid-block-map.ini")
                .expect("Custom ini should load");

        assert_eq!(
            config.block_map_path(),
            &PathBuf::from("tests/sandbox/block-non-existant.map")
        );

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn custom_config_invalid_socket() {
        init_logging();
        let config = Config::from_conf_file("tests/conf.d/invalid-socket.ini")
            .expect("Custom ini should load");

        assert_eq!(config.socket(), "socket:11210@127.0.0.1");

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn test_socket_address() {
        init_logging();
        assert!(is_socket_valid("inet:1234@localhost"));
        assert!(is_socket_valid("inet:1234@127.0.0.1"));

        assert!(!is_socket_valid(""));
        assert!(!is_socket_valid("inet:@127.0.0.1"));
        assert!(!is_socket_valid("1234@localhost"));
        assert!(!is_socket_valid("inet:1234localhost"));
        assert!(!is_socket_valid("inet:123@8.8.8.8"));
        assert!(!is_socket_valid("inet:222222@localhost"));
    }
}
