mod cli;
mod commands;
mod error;
mod prompts;
mod templates;
mod theme;

fn main() {
    let matches = cli::build_cli().get_matches();

    let result = match matches.subcommand() {
        Some(("init", sub)) => {
            let opts = commands::init::InitOptions {
                yes: sub.get_flag("yes"),
                force: sub.get_flag("force"),
            };
            commands::init::run(opts)
        }
        _ => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("❌ Error: {e}");
        std::process::exit(1);
    }
}
