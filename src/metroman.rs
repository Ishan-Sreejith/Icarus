use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
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
    #[command(name = "--out")]
    Out {
        file: String,
    },
    #[command(name = "train")]
    Train {
        #[arg(long, default_value = "training_data.json")]
        data: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        answer: String,
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
            Commands::Train { data, prompt, answer } => {
                let mut map: HashMap<String, String> = HashMap::new();
                if let Ok(raw) = fs::read_to_string(&data) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(obj) = parsed.as_object() {
                            for (k, v) in obj {
                                if let Some(s) = v.as_str() {
                                    map.insert(k.clone(), s.to_string());
                                }
                            }
                        } else if let Some(arr) = parsed.as_array() {
                            for entry in arr {
                                if let (Some(p), Some(a)) = (entry.get("prompt"), entry.get("answer")) {
                                    if let (Some(ps), Some(asv)) = (p.as_str(), a.as_str()) {
                                        map.insert(ps.to_string(), asv.to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                map.insert(prompt, answer);
                let json = serde_json::to_string_pretty(&map).unwrap();
                fs::write(&data, json).expect("Failed to write training data");
                println!("✓ Updated training data: {}", data);
            }
        }
    } else {
        println!("Metroman - CoRe Plugin Manager");
        println!("Usage: metroman --out <filename>");
        println!("       metroman train --data <file> --prompt <text> --answer <text>");
    }
}
