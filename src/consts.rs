pub mod arg {
    pub const ALLOW_MAP: &str = "allow-map";
    pub const BLOCK_MAP: &str = "block-map";
    pub const CONF: &str = "conf";
    pub const FOREGROUND: &str = "foreground";
    pub const GROUP: &str = "group";
    pub const LOGFILE: &str = "logfile";
    pub const PIDFILE: &str = "pidfile";
    pub const SOCKET: &str = "socket";
    pub const USER: &str = "user";
    pub const VERBOSE: &str = "verbose";
}

/// default conf values
pub mod default {
    pub const ALLOW_MAP: &str = "/etc/postkeeper/allow.map";
    pub const BLOCK_MAP: &str = "/etc/postkeeper/block.map";
    pub const CONFIG_PATH: &str = "/etc/postkeeper/postkeeper.ini";
    pub const LOGFILE: &str = "/var/log/postkeeper.log";
    pub const PIDFILE: &str = "/var/run/postkeeper/postkeeper.pid";
    pub const SOCKET: &str = "inet:11210@localhost";
}
