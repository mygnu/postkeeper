fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    // regex example without \ escape character
    // /^([[:space:]]*SERVICE_VERSION:).*/s//\1 1.0.0-alpha.8/
    // ^\ # start at the beginning of the line
    // Capture everything with in the braces ()
    // ([[:space:]]* # match any number of spaces
    // SERVICE_VERSION:\) # and `SERVICE_VERSION:`
    // this is first match
    // .*/ everything til the end of the line
    // this is second match an is ignored
    //
    // replacemint part
    // s// # sed substitute with
    // \1 {}/
    // \1 represents the first match
    // # {} gets filled with version number
    let ci_regexp = format!(
        "/^\\([[:space:]]*SERVICE_VERSION:\\).*/s//\\1 {}/",
        env!("CARGO_PKG_VERSION")
    );

    // auto update .gitlab-ci.yml version at build time
    std::process::Command::new("/bin/sed")
        // edit file in place
        .arg("-i")
        .arg(ci_regexp)
        .arg(".gitlab-ci.yml")
        .status()
        .expect("failed to execute sed");

    let cli_regexp = format!(
        "/^\\([[:space:]]*version:\\).*/s//\\1 {}/",
        env!("CARGO_PKG_VERSION")
    );

    // auto update cli.yml version at build time
    std::process::Command::new("/bin/sed")
        // edit file in place
        .arg("-i")
        .arg(&cli_regexp)
        .arg("src/cli.yml")
        .status()
        .expect("failed to execute sed");
}
