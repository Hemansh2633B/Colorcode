use colorcode::{Decoder, Encoder, EncoderOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"hello from ColorCode";
    let options = EncoderOptions::default();
    let code = Encoder::new(options).encode_bytes(data)?;
    code.matrix.save_png_with_quiet_zone(
        "examples/hello.c16.png",
        options.module_size,
        options.quiet_zone,
    )?;
    let decoded = Decoder::decode("examples/hello.c16.png")?;
    assert_eq!(decoded.data, data);
    println!(
        "wrote examples/hello.c16.png and decoded {} bytes",
        decoded.data.len()
    );
    Ok(())
}
