use clap::{Command as ClapCommand, Arg, ArgAction};

pub struct CliArgs {
    pub headless: bool,
    pub wait_on_failure: bool,
    pub wait: bool,
    pub visible: bool,
}

pub fn parse_arguments() -> CliArgs {
    let matches = ClapCommand::new("Integration Test")
        .arg(Arg::new("visible")
            .long("visible")
            .help("Run browser in visible mode (non-headless)")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("wait-on-failure")
            .long("wait-on-failure")
            .help("Wait for user input if the test fails")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("wait")
            .long("wait")
            .help("Wait for user input after the test, regardless of outcome")
            .action(ArgAction::SetTrue))
        .get_matches();

    CliArgs {
        headless: !matches.get_flag("visible"),
        wait_on_failure: matches.get_flag("wait-on-failure"),
        wait: matches.get_flag("wait"),
        visible: matches.get_flag("visible"),
    }
}
