use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use colorcode::{
    Compression, Decoder, DecoderOptions, EccLevel, Encoder, EncoderOptions,
    layout::payload_data_capacity_bytes,
};

#[derive(Debug, Parser)]
#[command(name = "colorcode")]
#[command(about = "Encode and decode 16-color matrix codes")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Encode {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = EccLevel::Low)]
        ecc: EccLevel,
        #[arg(long, default_value_t = Compression::None)]
        compression: Compression,
        #[arg(long, default_value_t = 16)]
        module_size: u32,
    },
    Decode {
        image: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 38.0)]
        tolerance: f32,
    },
    Capacity {
        #[arg(long, default_value_t = EccLevel::Low)]
        ecc: EccLevel,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Encode {
            input,
            output,
            ecc,
            compression,
            module_size,
        } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read input file {}", input.display()))?;
            let options = EncoderOptions {
                ecc_level: ecc,
                compression,
                module_size,
                ..EncoderOptions::default()
            };
            let code = Encoder::new(options).encode_bytes(&data)?;
            let output = output.unwrap_or_else(|| default_encoded_path(&input));
            code.matrix.save_png_with_quiet_zone(
                &output,
                options.module_size,
                options.quiet_zone,
            )?;
            eprintln!(
                "encoded {} bytes into {} (stored {} bytes, {} ECC nibbles, ecc={})",
                data.len(),
                output.display(),
                code.stored_payload_len,
                code.ecc_nibbles,
                ecc
            );
        }
        Command::Decode {
            image,
            output,
            tolerance,
        } => {
            let decoder = Decoder::new(DecoderOptions {
                color_tolerance: tolerance,
            });
            let decoded = decoder.decode_path(&image)?;
            if let Some(output) = output {
                fs::write(&output, &decoded.data)
                    .with_context(|| format!("failed to write {}", output.display()))?;
                eprintln!(
                    "decoded {} bytes into {} (ecc corrections: {})",
                    decoded.data.len(),
                    output.display(),
                    decoded.ecc_corrections
                );
            } else {
                std::io::stdout().write_all(&decoded.data)?;
            }
        }
        Command::Capacity { ecc } => {
            println!("{}", payload_data_capacity_bytes(ecc));
        }
    }
    Ok(())
}

fn default_encoded_path(input: &Path) -> PathBuf {
    let mut name = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("output")
        .to_string();
    name.push_str(".c16.png");
    input.with_file_name(name)
}
