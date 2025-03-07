# postkeeper

`postkeeper` is a [milter](https://en.wikipedia.org/wiki/Milter) application that filters emails
for `postfix` or `sendmail`. It provides basic functionality for implementing `allow` or `block`
lists per recipient.

## Building

On Debian and Ubuntu, install the package `libmilter-dev`.

```bash
# for rust binary
cargo build --release
# for debian package install `cargo install cargo-deb` and then
cargo deb
```

## How it works

See [DESIGN.md](DESIGN.md)

# Licence

Copyright 2020 Enhance Ltd <backend@enhance.com>
This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.
see [LICENCE](LICENSE)
