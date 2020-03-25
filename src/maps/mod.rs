//! Postkeeper global map management

mod map_parser;
use crate::config::global_conf;
use crate::prelude::*;
use lazy_static::lazy_static;
use map_parser::{last_modified, MapParser};
use std::{
    collections::HashMap,
    ops::Deref,
    path::Path,
    sync::RwLock,
    time::{Duration, SystemTime},
};

type PostKeepMap = RwLock<HashMap<String, Vec<String>>>;
type LastUpdatedTime = RwLock<SystemTime>;

// global objects are required due to `milter` crate nature of using callbacks.
lazy_static! {
    /// Holds a RwLock for allow hashmap in global state
    static ref ALLOW_MAP: PostKeepMap = RwLock::new(HashMap::new());

    /// Holds a RwLock for block hashmap in global state
    static ref BLOCK_MAP: PostKeepMap = RwLock::new(HashMap::new());

    /// Holds a RwLock for SystemTime timestamp when allow hashmap was last updated
    static ref ALLOW_MAP_LAST_UPDATED: LastUpdatedTime = RwLock::new(SystemTime::now());

    /// Holds a RwLock for SystemTime timestamp when block hashmap was last updated
    static ref BLOCK_MAP_LAST_UPDATED: LastUpdatedTime = RwLock::new(SystemTime::now());
}

/// reads the map file from given path,
/// parses and loads the data into global ALLOW_MAP
/// errors if doesn't have permissions to read the file
pub fn load_allow_map(path: impl AsRef<Path>) -> Result<()> {
    log::debug!("Loading allow map");
    let parser = MapParser::from_map_file(path)?;
    let mut allow_map = ALLOW_MAP.write().unwrap();
    *allow_map = parser.into_map();
    log::debug!("Finished loading {} allow maps", allow_map.deref().len());
    Ok(())
}

/// reads the map file from given path,
/// parses and loads the data into global BLOCK_MAP
/// errors if doesn't have permissions to read the file
pub fn load_block_map(path: impl AsRef<Path>) -> Result<()> {
    log::debug!("Loading block map");
    let parser = MapParser::from_map_file(path)?;
    let mut block_map = BLOCK_MAP.write().unwrap();
    *block_map = parser.into_map();
    log::debug!("Finished loading {} block maps", block_map.deref().len());
    Ok(())
}

/// tries to reload allow and block global maps
/// only if map files are modified and enough time passed since last reload
/// duration value is taken from global config.reload_interval
/// NOTE: this method can be called during a message process by the milter
/// therefore Errors are simply logged, allowing the milter process to succeed
pub fn load_maps_if_changed() {
    let allow_map_path = global_conf().allow_map_path();
    let block_map_path = global_conf().block_map_path();
    let allow_last_updated = allow_last_updated();
    let block_last_updated = block_last_updated();
    let reload_interval = global_conf().reload_interval();

    if should_update(allow_map_path, allow_last_updated, reload_interval) {
        if let Err(e) = load_allow_map(allow_map_path) {
            log::error!("{}", e);
        } else {
            // update the global last_updated time
            let mut last_updated = ALLOW_MAP_LAST_UPDATED.write().unwrap();
            *last_updated = SystemTime::now();
            log::debug!("Successfully Reloaded Allow Map");
        }
    }

    if should_update(block_map_path, block_last_updated, reload_interval) {
        if let Err(e) = load_block_map(block_map_path) {
            log::error!("{}", e);
        } else {
            // update the global last_updated time
            let mut last_updated = BLOCK_MAP_LAST_UPDATED.write().unwrap();
            *last_updated = SystemTime::now();
            log::debug!("Successfully Reloaded Block Map");
        }
    }
}

/// checks the conditions if a map should be updated, returns bool
/// reads the last modified time from given path
/// compares it with last_updated time
/// returns true if it is grater than the reload interval
fn should_update(
    path: impl AsRef<Path>,
    last_updated: SystemTime,
    reload_interval: Duration,
) -> bool {
    if let Ok(elapsed) = last_updated.elapsed() {
        log::trace!("Time elapsed since last reload {:?}", elapsed);

        if elapsed >= reload_interval {
            match last_modified(path.as_ref()) {
                Ok(modified) => modified > last_updated,
                Err(e) => {
                    log::error!("Error checking {:?} metadata, {:?}", path.as_ref(), e);
                    false
                }
            }
        } else {
            log::trace!(
                "Skipped Loading Block Map, not enough time elapsed {:?}",
                elapsed
            );
            false
        }
    } else {
        log::warn!("System Time is skewed!");
        false
    }
}

/// query the global BLOCK_MAP to match if
/// given recipient has blocked the sender
pub fn is_blocked(recipient: &str, sender: &str) -> bool {
    let recipient = recipient.to_lowercase();

    log::trace!(
        "trying to find block match for recpt: {}, sender: {}",
        recipient,
        sender
    );

    let map = BLOCK_MAP.read().unwrap();

    if let Some(senders) = map.get(&recipient) {
        senders.iter().any(|s| s.eq_ignore_ascii_case(sender))
    } else {
        false
    }
}

/// query the global ALLOW_MAP to match if
/// given recipient has the sender in allow-list
pub fn is_allowed(recipient: &str, sender: &str) -> bool {
    let recipient = recipient.to_lowercase();

    log::info!(
        "trying to find allow match for recpt: {}, sender: {}",
        recipient,
        sender
    );

    let map = ALLOW_MAP.read().unwrap();

    if let Some(senders) = map.get(&recipient) {
        senders.iter().any(|s| s.eq_ignore_ascii_case(sender))
    } else {
        false
    }
}

/// return a copy of global SystemTime
fn allow_last_updated() -> SystemTime {
    let last_updated = ALLOW_MAP_LAST_UPDATED.read().unwrap();

    last_updated.deref().to_owned()
}

/// return a copy of global SystemTime
fn block_last_updated() -> SystemTime {
    let last_updated = BLOCK_MAP_LAST_UPDATED.read().unwrap();

    last_updated.deref().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::ops::{Add, Sub};
    use std::sync::Once;

    static PREP_TEST: Once = Once::new();
    /// Load the maps only the first time this method is called.
    fn load_maps() {
        PREP_TEST.call_once(|| {
            assert_eq!(load_allow_map("tests/test_allow.map"), Ok(()));
            assert_eq!(load_block_map("tests/test_block.map"), Ok(()));
        });
    }
    #[test]
    fn test_allow_map() {
        load_maps();
        // first map of the file
        assert!(is_allowed("teresa@example.com", "taurean@example.org"));
        assert!(is_allowed("haskell@example.com", "yvette@example.net"));
        assert!(is_allowed("haskell@example.com", "cloyd@example.com"));
        assert!(is_allowed("vida@example.net", "cindy@example.org"));

        // last map of the file
        assert!(is_allowed("vida@example.net", "eusebio@example.com"));

        // this data doesn't exist
        assert!(!is_allowed("elmo@example.net", "cloyd@example.com"));
        assert!(!is_allowed("elmo33@example.net", "test@example.com"));
        assert!(!is_allowed("pat@example.net", "cloyd@example.com"));
        assert!(!is_allowed("milter@example.net", "example@example.com"));
        assert!(!is_allowed("hello@example.net", "hi@example.com"));
    }

    #[test]
    fn test_block_map() {
        load_maps();
        // first map first and last match
        assert!(is_blocked("reanna@example.com", "kale@example.org"));
        assert!(is_blocked("reanna@example.com", "maximillia@example.net"));

        // last map firest and last match
        assert!(is_blocked("bertrand@example.org", "summer@example.com"));
        assert!(is_blocked("bertrand@example.org", "griffin@example.net"));
    }

    #[test]
    fn test_should_update() {
        let path = "tests/test.map";
        let last_updated = SystemTime::now().sub(Duration::from_secs(10));
        fs::File::create(path).unwrap();

        // enough time has passed should update
        assert!(should_update(path, last_updated, Duration::from_secs(0)));
        assert!(should_update(path, last_updated, Duration::from_secs(3)));
        assert!(should_update(path, last_updated, Duration::from_secs(8)));
        assert!(should_update(path, last_updated, Duration::from_secs(10)));

        // still need to wait, shouldn't update
        assert!(!should_update(path, last_updated, Duration::from_secs(11)));
        assert!(!should_update(path, last_updated, Duration::from_secs(12)));
        assert!(!should_update(path, last_updated, Duration::from_secs(13)));
        assert!(!should_update(path, last_updated, Duration::from_secs(14)));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_should_update_fail() {
        let path = "tests/test2.map";
        let last_updated = SystemTime::now().sub(Duration::from_secs(10));

        // enough time has passed but file does not exist, shouldn't update
        assert!(!should_update(path, last_updated, Duration::from_secs(0)));
        assert!(!should_update(path, last_updated, Duration::from_secs(3)));
        assert!(!should_update(path, last_updated, Duration::from_secs(8)));
        assert!(!should_update(path, last_updated, Duration::from_secs(10)));
        assert!(!should_update(path, last_updated, Duration::from_secs(11)));
        assert!(!should_update(path, last_updated, Duration::from_secs(12)));
        assert!(!should_update(path, last_updated, Duration::from_secs(13)));
        assert!(!should_update(path, last_updated, Duration::from_secs(14)));
    }

    #[test]
    fn test_should_update_fail_skewed_time() {
        let path = "tests/test3.map";
        // skew time with last updated in future
        let last_updated = SystemTime::now().add(Duration::from_secs(10));
        fs::File::create(path).unwrap();

        // last updated time is skewed, shouldn't update
        assert!(!should_update(path, last_updated, Duration::from_secs(0)));
        assert!(!should_update(path, last_updated, Duration::from_secs(3)));
        assert!(!should_update(path, last_updated, Duration::from_secs(8)));
        assert!(!should_update(path, last_updated, Duration::from_secs(10)));
        assert!(!should_update(path, last_updated, Duration::from_secs(11)));
        assert!(!should_update(path, last_updated, Duration::from_secs(12)));
        assert!(!should_update(path, last_updated, Duration::from_secs(13)));
        assert!(!should_update(path, last_updated, Duration::from_secs(14)));

        fs::remove_file(path).unwrap();
    }
}
