use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rustkyl", about = "A static site generator for DataTalks.Club")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the static site
    Build,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build) => {
            println!("Building site... (not yet implemented)");
        }
        None => {
            println!("Hello from rustkyl! Use --help to see available commands.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parses_no_args() {
        let cli = Cli::try_parse_from(["rustkyl"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parses_build_subcommand() {
        let cli = Cli::try_parse_from(["rustkyl", "build"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert!(matches!(cli.command, Some(Commands::Build)));
    }

    #[test]
    fn test_cli_rejects_unknown_flag() {
        let result = Cli::try_parse_from(["rustkyl", "--nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_help_flag() {
        let result = Cli::try_parse_from(["rustkyl", "--help"]);
        // --help causes clap to return an error (it's a special exit)
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }
}
