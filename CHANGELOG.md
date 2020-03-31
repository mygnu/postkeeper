# Changelog 

## [0.3.1] 2020-03-31
### Changed
- reduce verbosity in System-V init script
- store dev binaries with version string

## [0.3.0] 2020-03-24

### Added 
- create logfile if doesn't exist
- open logfile in append mode to avoid seeking cursor

### Changed
- dependencies updated
- logging updated (after some live testing)
- better comments in config file
- remove logfile existence testing

## [0.2.0] 2020-03-XX

### Added
- milter implementation, takes action on matching from block list and adds header on allow list match
- config/cli option for Action on block list match `reject`, `discard` or `accept`
- load allow/block maps in memory and update them
- config option to set minimum reload_interval before update maps in memory
- cli option `--test-config` or `-t` test configuration and exit
- maps are only updated if `reload_interval` time has passed and map files are modified

### Changed
- logging level can also be defined in `postkeeper.ini` and is read from there first


### Bug fixes
- set user permissions to `postkeeper` group on deb install

## [0.1.1] 2020-03-XX

### Added
- Provide default config and map files
- Cli arguments and daemon/milter configuration implemented
- Config validation implemented
- upload `.deb` package to gcp bucket 
