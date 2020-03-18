//! Postkeeper milter map parser implementation

use crate::prelude::*;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    time::SystemTime,
};

/// Each map value starts at the beginning of the line
/// Multiline maps are allowed as consequent lines start with whitespace character[s].
/// line text starting with `#` treated as comment and ignored. either at the beginning of the line
/// and also in multiline context
/// comments are not permitted on lines with data
///
/// values are treated case insensitive
/// EXAMPLE:
///  teresa@example.com gay@example.com candice@example.net cornelius@example.net jarret@example.org zachariah@example.org wilfred@example.com
///     # this is allowed comment
///     hildegard@example.com taurean@example.org
///  alayna@example.com claude@example.net stephan@example.net
///     jordan@example.net
///     juston@example.com
#[derive(Debug)]
pub struct MapParser {
    map: HashMap<String, Vec<String>>,
}

impl MapParser {
    /// parsese the map file from given path into inner hashamp
    pub fn from_map_file(path: impl AsRef<Path>) -> Result<Self> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let mut parser = Self {
            map: HashMap::new(),
        };
        // buffer to hold a single logical map line
        // we need this as map files can define values over multiple lines
        let mut line_buf = String::new();

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let trimmed = line.trim();
                    // skip over empty or comment lines
                    if trimmed.starts_with('#') || trimmed.is_empty() {
                        continue;
                    }
                    // if last line has been processed or we are still on the same logical line
                    // and it starts with whitespace characters
                    if line_buf.is_empty() || line.starts_with(char::is_whitespace) {
                        // add line to buffer
                        line_buf.push_str(&line);
                    } else {
                        // process the current logical line and clear the buffer
                        parser.process_line(&line_buf);
                        line_buf.clear();
                        debug_assert!(line_buf.is_empty());

                        // at this point previous logical line has been processed
                        // add the current line to buffer
                        line_buf.push_str(&line);
                    }
                }
                Err(e) => {
                    log::error!("Could not read next line with error {:?}", e);
                }
            }
        }
        // process the last line in buffer
        parser.process_line(&line_buf);
        Ok(parser)
    }

    /// consumes the parser and returns the inner parsed HashMap
    pub fn into_map(self) -> HashMap<String, Vec<String>> {
        self.map
    }

    // process a logical single line of map
    // and insert parsed values to HashMap
    fn process_line(&mut self, line: &str) {
        let list: Vec<&str> = line.split_ascii_whitespace().collect();

        // there must be at least two values in a map
        if list.len() < 2 {
            log::warn!("Skip parsing, line only contains one value");
            return;
        }

        if let Some((head, tail)) = list.split_first() {
            // lowercase recipient email before inserting
            let key = (*head).to_lowercase();
            // values are inserted as is (lowercased on check-time)
            let value = tail.iter().map(|v| (*v).to_owned()).collect();
            self.map.insert(key, value);
        }
    }
}

/// check if file has been modified within the duration
/// errors if cannot read file matadata or time elapsed is somehow negative
pub fn last_modified(path: impl AsRef<Path>) -> Result<SystemTime> {
    fs::metadata(path)
        .map_err(Error::from)?
        .modified()
        .map_err(Error::from)
}
