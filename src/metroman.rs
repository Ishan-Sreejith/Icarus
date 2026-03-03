use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[command(name = "metroman")]
#[command(about = "CoRe Language Plugin Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new plugin syntax file
    #[command(name = "--out")]
    Out {
        /// Output filename (e.g. myplugin.fr)
        file: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct PluginSyntax {
    pub name: String,
    pub version: String,
    pub functions: HashMap<String, String>, // name -> implementation snippet
}

fn main() {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Out { file } => {
                let plugin = PluginSyntax {
                    name: "my-plugin".to_string(),
                    version: "1.0.0".to_string(),
                    functions: HashMap::from([(
                        "custom-thing".to_string(),
                        "fn custom-thing: value { say: value }".to_string(),
                    )]),
                };

                let json = serde_json::to_string_pretty(&plugin).unwrap();
                let mut f = File::create(&file).expect("Failed to create file");

                writeln!(f, "// Metroman Plugin Definition").unwrap();
                writeln!(f, "// Define your custom functions here").unwrap();
                writeln!(f, "{}", json).unwrap();

                println!("✓ Created plugin template: {}", file);
            }
        }
    } else {
        println!("Metroman - CoRe Plugin Manager");
        println!("Usage: metroman --out <filename>");
    }
}
