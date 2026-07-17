use colorcode::layout;
use colorcode::{
    Compression, Decoder, DecoderOptions, EccLevel, Encoder, EncoderOptions, renderer,
};

#[test]
fn matrix_roundtrip_text() {
    let data = b"ColorCode roundtrip through the in-memory matrix.";
    let code = Encoder::encode(data).unwrap();
    let decoded = Decoder::new(DecoderOptions::default())
        .decode_matrix(&code.matrix)
        .unwrap();
    assert_eq!(decoded.data, data);
    assert!(decoded.crc32_valid);
}

#[test]
fn png_roundtrip_binary() {
    let data = (0..180).map(|v| (v * 37 % 256) as u8).collect::<Vec<_>>();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("payload.c16.png");
    let options = EncoderOptions {
        ecc_level: EccLevel::Medium,
        module_size: 12,
        ..EncoderOptions::default()
    };
    let code = Encoder::new(options).encode_bytes(&data).unwrap();
    renderer::save_png(&code.matrix, &path, options.module_size, options.quiet_zone).unwrap();
    let decoded = Decoder::decode(&path).unwrap();
    assert_eq!(decoded.data, data);
}

#[test]
fn rotated_png_roundtrip() {
    let data = b"rotation-safe decode path";
    let code = Encoder::encode(data).unwrap();
    let image = renderer::render_png(&code.matrix, 10, 4);
    let rotated = image::imageops::rotate90(&image);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rotated.png");
    rotated.save(&path).unwrap();
    let decoded = Decoder::decode(&path).unwrap();
    assert_eq!(decoded.data, data);
}

#[test]
fn jpeg_roundtrip_with_color_classifier() {
    let data = b"jpeg introduces color drift, so decoding uses Lab Delta-E.";
    let code = Encoder::encode(data).unwrap();
    let image = renderer::render_png(&code.matrix, 18, 4);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    image.save(&path).unwrap();
    let decoded = Decoder::new(DecoderOptions {
        color_tolerance: 45.0,
    })
    .decode_path(&path)
    .unwrap();
    assert_eq!(decoded.data, data);
}

#[test]
fn ecc_repairs_payload_symbol_corruption() {
    let data = b"single symbol corruption is recoverable at low ECC";
    let mut code = Encoder::encode(data).unwrap();
    let first_payload = layout::payload_cells()[0];
    let old = code.matrix.get(first_payload.row, first_payload.col);
    code.matrix
        .set(first_payload.row, first_payload.col, (old + 7) & 0x0f);
    let decoded = Decoder::new(DecoderOptions::default())
        .decode_matrix(&code.matrix)
        .unwrap();
    assert_eq!(decoded.data, data);
    assert!(decoded.ecc_corrections > 0);
}

#[test]
fn compression_roundtrip() {
    let data = vec![b'A'; 240];
    let options = EncoderOptions {
        compression: Compression::Lz4,
        ecc_level: EccLevel::High,
        ..EncoderOptions::default()
    };
    let code = Encoder::new(options).encode_bytes(&data).unwrap();
    let decoded = Decoder::new(DecoderOptions::default())
        .decode_matrix(&code.matrix)
        .unwrap();
    assert_eq!(decoded.data, data);
    assert_eq!(decoded.metadata.compression, Compression::Lz4);
}
