//! x940: MT940 bank statement transformer CLI

use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use x940rs::{parse_mt940, to_camt053, to_csv, to_json, DecoderChain};

#[derive(Parser)]
#[command(
    name = "x940",
    version,
    about = "Transform SWIFT MT940 bank statements into JSON, CSV, and camt.053 XML",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and transform MT940 files
    Transform {
        /// Input MT940 file path (reads from stdin if omitted)
        input: Option<PathBuf>,

        /// Output format(s): json, csv, camt053 (comma-separated for multiple)
        #[arg(short, long, value_delimiter = ',')]
        format: Vec<OutputFormat>,

        /// Output file path (for single-format exports)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output file prefix for multi-format exports: produces prefix.json, prefix.csv, prefix.xml
        #[arg(long = "output-prefix")]
        output_prefix: Option<String>,

        /// Tag 86 dialect resolver: auto, swift, gvc, angular
        #[arg(short, long, default_value = "auto")]
        resolver: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Json,
    Csv,
    Camt053,
}

fn read_input(path: Option<PathBuf>) -> io::Result<String> {
    match path {
        Some(p) => std::fs::read_to_string(p),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn write_output(content: &str, path: Option<&PathBuf>) -> io::Result<()> {
    match path {
        Some(p) => std::fs::write(p, content),
        None => {
            io::stdout().write_all(content.as_bytes())?;
            Ok(())
        }
    }
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Transform {
            input,
            format,
            output,
            output_prefix,
            resolver,
        } => {
            if format.is_empty() {
                eprintln!("Error: at least one --format is required (json, csv, camt053)");
                std::process::exit(1);
            }

            let raw = read_input(input).unwrap_or_else(|e| {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            });

            let chain = DecoderChain::with_resolver(&resolver).unwrap_or_else(|| {
                eprintln!("Warning: unknown resolver '{}', falling back to auto", resolver);
                DecoderChain::auto()
            });

            let statements = match parse_mt940(&raw, &chain) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            };

            let multi = format.len() > 1 || output_prefix.is_some();
            let prefix = output_prefix.as_deref().unwrap_or("export");

            for fmt in &format {
                let result = match fmt {
                    OutputFormat::Json => to_json(&statements),
                    OutputFormat::Csv => to_csv(&statements),
                    OutputFormat::Camt053 => to_camt053(&statements),
                };

                let content = result.unwrap_or_else(|e| {
                    eprintln!("Export error: {}", e);
                    std::process::exit(1);
                });

                if multi {
                    let ext = match fmt {
                        OutputFormat::Json => "json",
                        OutputFormat::Csv => "csv",
                        OutputFormat::Camt053 => "xml",
                    };
                    let path = PathBuf::from(format!("{}.{}", prefix, ext));
                    if let Err(e) = write_output(&content, Some(&path)) {
                        eprintln!("Error writing {}: {}", path.display(), e);
                        std::process::exit(1);
                    }
                } else {
                    if let Err(e) = write_output(&content, output.as_ref()) {
                        eprintln!("Error writing output: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
