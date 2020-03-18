# Design

We are going to use `milter` crate for setting up the milter, `milter` [crate](https://docs.rs/milter/) is in turn a rust wrapper around `libmilter` library by `sendmail` maintainers. libmilter takes care of communication to `MTA` via a unix socket file and/or a tcp socket (allows us to run postkeeper in a separate docker container).

## installation
`postkeeper` is installed in `/usr/sbin/postkeeper`. Main config files will be stored in `/etc/postkeeper/` and `rundir` would be `/var/run/postkeeper/`. `postkeeper` has three main config files, daemon configuration lives in `postkeeper.ini` (ini format) and two map files `allow.map` and `block.map`. Running in daemon mode will create `postkeeper.pid` and using unix sockets will crate `postkeeper.sock` in the `rundir`.

## debian package
debian package is provided for installation.
### Pros
- preform checks for dependencies before installing (currently depends: libc6 (>= 2.27), libmilter1.0.1 (>= 8.15.2), procps, libgcc1 (>= 1:8.3.0))
- creates required directory structures
- setup default configuration on first install but doesn't overwrite on update (configurable)
- creates default `user:group` as we'd want to run this as non-root for security

### Cons
- n/a at this stage

Users should be able to run `postkeeps` under a low privileged user rather than root, (as long as you are using unix sockets to communicate or socket file is accessable to the MTA `postfix`)

Map file format resembles key value pairs where key is the recipient email address and value is a whitespace seperated list of emails address. Map file (see below) will be used to match against a message's sender and recipient to block/allow that message.

`postkeeper` will keep both maps in memory for faster processing and perodically check for file changes (presumably at the start of email processing) and reload maps to memory for the if map file is modified since last check. (TBD: later we may dump maps periodically in binary file for faster reload/restart given the text map files are not modified)



## allow/block maps
having two map files allows to load/save each map independent of each other. Proposed format for map files:

```
# comment is ignored
recipient@email.com nasty.work@email.com stalker@email.com racist.uncle@email.com
```

or 
```
recipient@email.com 
    nasty.work@email.com 
    # comment here is also ignored
    stalker@email.com 
    racist.uncle@email.com
```

single map value starts at the beginning of the line, Multiline maps are allowed as consequent lines start with a whitespace character.

line text starting with `#` treated as comment and ignored.


## emails headers
Each processed email will get inserted a header `X-Postkeeper-Allow: Yes` if the recipient of the email has put the sender in `allow` list otherwise email will simply get blocked if sender is in `block` list for the recipient. No header is inserted if email doesn't match any allow/block lists.

## dependencies 
- libmilter-dev `apt install libmilter-dev`

# TBD
- testing on live server 
- how to implement integration tests (possible using [miltertest](http://manpages.org/miltertest/8))
- config file parameters
- cli arguments list
- how to bypass `rspamd` if sender is in `allow` list


## bench
After some initial benchmarking for loading map of 1_000_000 lists with 8 entries each (some multi lined), we have the following numbers.

|                   | allow.map     | allow.bin     |
|------------------ | ------------- |---------------|
| load from disk    | 2941ms        | 589ms         | 
| size on disk      | 169M          | 122M          | 
| search record     | 1ms           | 1ms           |
