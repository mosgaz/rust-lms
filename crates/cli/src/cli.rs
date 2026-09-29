use clap::{Arg, ArgAction, Command};

pub fn build_cli() -> Command {
    Command::new("rust-kit")
        .about("Rust Kit UI CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("init")
                .about("Initialize styles in the current project")
                .arg(
                    Arg::new("yes")
                        .short('y')
                        .long("yes")
                        .help("Skip prompts and accept defaults")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("Overwrite existing files without prompting")
                        .action(ArgAction::SetTrue),
                ),
        )
}