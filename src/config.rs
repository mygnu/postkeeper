use crate::consts::*;
use crate::prelude::*;
use clap::ArgMatches;
use ini::Ini;
use std::path::{Path, PathBuf};

/// holds Configuration for milter and daemon
#[derive(Debug, Default)]
pub struct Config {
    pid_file: PathBuf,
    log_file: PathBuf,
    allow_map: PathBuf,
    block_map: PathBuf,
    socket: String,
    user: Option<String>,
    group: Option<String>,
}

impl Config {
    /// genarate Config from CLI arguments
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

        trace!("Config Loaded: {:#?}", &conf);

        Ok(conf)
    }

    /// validate configuration, daemon and milter should be able to run if
    /// validation returns without an error
    pub fn validate(&self) -> Result<()> {
        use Validation::*;
        // assume valid state before performing checks
        let mut state = Valid;

        if !is_socket_valid(self.socket()) {
            error!("{:?} is not a valid socket", self.socket());
            state = Invalid
        }

        trace!("Socket is valid");

        if !self.allow_map().is_file() {
            error!("{:?} is not a valid file", self.allow_map());
            state = Invalid
        }
        trace!("allow.map is valid");

        if !self.log_file().is_file() {
            error!("{:?} is not a valid file", self.log_file());
            state = Invalid
        }
        trace!("log file is valid");

        if !self.block_map().is_file() {
            error!("{:?} is not a valid file", self.block_map());
            state = Invalid
        }
        trace!("block.map is valid");

        if state == Invalid {
            Err(Error::config_err("Invalid Configuration!"))
        } else {
            trace!("Config validation finished with success.");
            Ok(())
        }
    }

    pub fn allow_map(&self) -> &PathBuf {
        &self.allow_map
    }
    pub fn block_map(&self) -> &PathBuf {
        &self.block_map
    }
    pub fn pid_file(&self) -> &PathBuf {
        &self.pid_file
    }

    pub fn log_file(&self) -> &PathBuf {
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

    /// builds config from config ini path
    /// uses default values if not defined in the config
    /// to allow only define variable that require a change
    /// NOTE:
    /// Errors if cannot read config file
    fn from_conf_file(path: impl AsRef<Path>) -> Result<Self> {
        trace!("loading config from path {:?}", path.as_ref());

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

        Ok(Self {
            allow_map,
            block_map,
            pid_file,
            log_file,
            socket,
            user,
            group,
        })
    }
}

// holds config validation state
#[derive(PartialEq)]
enum Validation {
    Valid,
    Invalid,
}

// validates socket address for format and connectivity
fn is_socket_valid(socket: &str) -> bool {
    trace!("validating socket {}", socket);
    // example socket `inet:11210@127.0.0.1`
    let parts: Vec<&str> = socket.split(':').collect();

    // socket must start with `inet`
    if parts.get(0) != Some(&"inet") {
        trace!("socket must start with `inet`");
        return false;
    }
    let addr = parts.get(1);
    if addr.is_none() {
        return false;
    }
    let addr: Vec<&str> = addr.unwrap_or_else(|| &"").split('@').collect();

    let tcp_addr = format!(
        "{}:{}",
        addr.get(1).unwrap_or(&""),
        addr.get(0).unwrap_or(&"")
    );

    trace!("Checking connectivity to `{}`", tcp_addr);

    // check if you can connect to the socket
    std::net::TcpListener::bind(tcp_addr).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_config_must_load() {
        // default postkeeper must load
        let config = Config::from_conf_file("assets/etc/postkeeper.ini")
            .expect("Default postkeeper.ini is should load");

        assert_eq!(config.pid_file(), &PathBuf::from(default::PIDFILE));
        assert_eq!(config.log_file(), &PathBuf::from(default::LOGFILE));

        assert_eq!(config.socket(), default::SOCKET);

        assert_eq!(config.allow_map(), &PathBuf::from(default::ALLOW_MAP));

        assert_eq!(config.block_map(), &PathBuf::from(default::BLOCK_MAP));

        assert_eq!(config.user(), Some("postkeeper"));
        assert_eq!(config.group(), Some("postkeeper"));
    }

    #[test]
    fn non_existant_config() {
        let err =
            Config::from_conf_file("non-existant.ini").expect_err("Config file should not load");
        assert_eq!(
            err,
            Error::with_msg(Kind::ConfigError, "Config file not found")
        )
    }

    #[test]
    fn custom_config_values() {
        let config =
            Config::from_conf_file("tests/conf.d/valid.ini").expect("Custom ini is should load");

        assert_eq!(
            config.pid_file(),
            &PathBuf::from("tests/sandbox/postkeeper.pid")
        );
        assert_eq!(
            config.log_file(),
            &PathBuf::from("tests/sandbox/postkeeper.log")
        );

        assert_eq!(config.socket(), "inet:11210@127.0.0.1");

        assert_eq!(
            config.allow_map(),
            &PathBuf::from("tests/sandbox/allow.map")
        );

        assert_eq!(
            config.block_map(),
            &PathBuf::from("tests/sandbox/block.map")
        );

        assert_eq!(config.validate(), Ok(()))
    }

    #[test]
    fn custom_config_invalid_allow_map() {
        let config = Config::from_conf_file("tests/conf.d/invalid-allow-map.ini")
            .expect("Custom ini is should load");

        assert_eq!(
            config.allow_map(),
            &PathBuf::from("tests/sandbox/allow-non-existant.map")
        );

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn custom_config_invalid_block_map() {
        let config = Config::from_conf_file("tests/conf.d/invalid-block-map.ini")
            .expect("Custom ini is should load");

        assert_eq!(
            config.block_map(),
            &PathBuf::from("tests/sandbox/block-non-existant.map")
        );

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn custom_config_invalid_socket() {
        let config = Config::from_conf_file("tests/conf.d/invalid-socket.ini")
            .expect("Custom ini is should load");

        assert_eq!(config.socket(), "socket:11210@127.0.0.1");

        assert_eq!(
            config.validate(),
            Err(Error::config_err("Invalid Configuration!"))
        )
    }

    #[test]
    fn socket_address() {
        assert!(is_socket_valid("inet:1234@localhost"));
        assert!(is_socket_valid("inet:1234@127.0.0.1"));

        assert!(!is_socket_valid(""));
        assert!(!is_socket_valid("1234@localhost"));
        assert!(!is_socket_valid("inet:1234localhost"));
        assert!(!is_socket_valid("inet:123@8.8.8.8"));
        assert!(!is_socket_valid("inet:222222@localhost"));
    }
}
