use clap::{Parser, Subcommand};
use jsonschema_rs::Validator;
use std::fs;
use std::path::PathBuf;
use std::process;

/// 基于 Rust 的高性能 JSON Schema 校验工具，支持 Draft 2020-12。
#[derive(Parser)]
#[command(name = "jsonschema-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 校验 JSON 实例是否符合 JSON Schema。
    Validate {
        /// JSON Schema 文件路径。
        #[arg(short, long)]
        schema: PathBuf,

        /// 待校验的 JSON 实例文件路径。
        #[arg(short, long)]
        data: PathBuf,

        /// 输出格式：`text`（默认）或 `json`。
        #[arg(long, default_value = "text")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Validate {
            schema,
            data,
            output,
        } => {
            let schema_str =
                fs::read_to_string(&schema).unwrap_or_else(|e| {
                    eprintln!("Error reading schema file {}: {}", schema.display(), e);
                    process::exit(1);
                });

            let schema_value: serde_json::Value =
                serde_json::from_str(&schema_str).unwrap_or_else(|e| {
                    eprintln!("Error parsing schema JSON: {}", e);
                    process::exit(1);
                });

            let data_str = fs::read_to_string(&data).unwrap_or_else(|e| {
                eprintln!("Error reading data file {}: {}", data.display(), e);
                process::exit(1);
            });

            let instance: serde_json::Value =
                serde_json::from_str(&data_str).unwrap_or_else(|e| {
                    eprintln!("Error parsing data JSON: {}", e);
                    process::exit(1);
                });

            let validator = Validator::new(schema_value);
            let errors = validator.iter_errors(&instance);

            match output.as_str() {
                "json" => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&errors).unwrap()
                    );
                }
                _ => {
                    if errors.is_empty() {
                        println!("\u{2713} Valid");
                        process::exit(0);
                    } else {
                        eprintln!(
                            "\u{2717} Invalid — {} error(s):",
                            errors.len()
                        );
                        for (i, err) in errors.iter().enumerate() {
                            eprintln!("  {}. {}", i + 1, err);
                        }
                        process::exit(1);
                    }
                }
            }
        }
    }
}
