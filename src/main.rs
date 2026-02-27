mod cli;
mod config;
mod error;
mod link;
mod node;
mod vault;
mod vcs;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    match cli::execute(&cli) {
        Ok(output) => {
            if !output.is_empty() {
                print!("{}", output);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
