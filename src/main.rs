use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use colorcode::{
    self, ALL_VERSIONS, Compression, Decoder, DecoderOptions, EccLevel, Encoder, EncoderOptions,
    Version, DEFAULT_QUIET_ZONE, layout::payload_data_capacity_bytes,
};

/// Maximum allowed PNG output size, in bytes.
const DEFAULT_PNG_BUDGET_BYTES: u64 = 5 * 1024 * 1024;

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
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value_t = DEFAULT_PNG_BUDGET_BYTES)]
        png_budget_bytes: u64,
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
            version,
            png_budget_bytes,
        } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read input file {}", input.display()))?;
            let _version = match version.as_deref() {
                Some(text) => Some(Version::parse(text).ok_or_else(|| {
                    anyhow::anyhow!("unknown version '{text}'; expected v2 or 2")
                })?),
                None => None,
            };
            let options = EncoderOptions {
                ecc_level: ecc,
                compression,
                module_size,
                quiet_zone: DEFAULT_QUIET_ZONE,
                ..EncoderOptions::default()
            };
            let code = Encoder::new(options).encode_bytes(&data)?;
            let output_path = output.unwrap_or_else(|| default_encoded_path(&input));
            let (effective_module_size, image_bytes) = write_png_within_budget(
                &code,
                &output_path,
                options.module_size,
                options.quiet_zone,
                png_budget_bytes,
            )?;
            eprintln!(
                "encoded {} bytes into {} (stored {} bytes, {} ECC nibbles, version={}, ecc={}, module_size={}, png={:.2} MB)",
                data.len(),
                output_path.display(),
                code.stored_payload_len,
                code.ecc_nibbles,
                code.matrix.size,
                ecc,
                effective_module_size,
                image_bytes as f64 / 1_048_576.0,
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
            println!("version,size,ecc_level,capacity_bytes");
            for &v in ALL_VERSIONS {
                let n = v.size();
                let cap = payload_data_capacity_bytes(ecc);
                println!("{},{},{:?},{}", format!("{v:?}").to_lowercase(), n, ecc, cap);
            }
        }
    }
    Ok(())
}

/// Render the matrix to `path` while staying within the configured PNG byte
/// budget. Returns the module size actually used and the resulting file size.
fn write_png_within_budget(
    code: &colorcode::EncodedCode,
    path: &Path,
    requested_module_size: u32,
    quiet_zone: u32,
    budget_bytes: u64,
) -> anyhow::Result<(u32, u64)> {
    let mut module_size = requested_module_size.max(1);
    loop {
        let image = code.matrix.render_png_with_quiet_zone(module_size, quiet_zone);
        let buf = encode_png_to_vec(&image)?;
        if buf.len() as u64 <= budget_bytes || module_size == 1 {
            std::fs::write(path, &buf)
                .with_context(|| format!("failed to write {}", path.display()))?;
            return Ok((module_size, buf.len() as u64));
        }
        module_size = module_size.saturating_sub(1).max(1);
    }
}

fn encode_png_to_vec(image: &image::RgbImage) -> anyhow::Result<Vec<u8>> {
    use image::ImageEncoder;
    let mut buf = Vec::with_capacity(image.len() * 3);
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf)
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
