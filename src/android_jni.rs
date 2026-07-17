use jni::objects::{JClass, JObject, JByteArray, JIntArray, JLong};
use jni::sys::{jint, jlong, jfloat, jboolean, jbyte};
use jni::JNIEnv;
use colorcode::{
    Compression, DecodedPayload, Decoder, DecoderOptions, EccLevel, Encoder, EncoderOptions,
};
use std::ptr;

/// Result class name for JNI
const RESULT_CLASS: &str = "com/example/colorcode/DecodeResult";
const METADATA_CLASS: &str = "com/example/colorcode/Metadata";

#[no_mangle]
pub extern "system" fn Java_com_example_colorcode_ColorCodeNative_nativeInit(
    _env: JNIEnv,
    _class: JClass,
    tolerance: jfloat,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("ColorCodeDecoder"),
    );

    let options = DecoderOptions {
        color_tolerance: tolerance,
    };
    let decoder = Box::new(Decoder::new(options));
    Box::into_raw(decoder) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_example_colorcode_ColorCodeNative_nativeDecodeImage(
    mut env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
    image_data: JByteArray,
    _width: jint,
    _height: jint,
) -> JObject {
    let decoder = unsafe { &mut *(decoder_ptr as *mut Decoder) };

    let data: Vec<u8> = env.convert_byte_array(&image_data).unwrap_or_default();

    let img = match image::load_from_memory(&data) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return JObject::null();
        }
    };

    let result = match decoder.decode_path(&img) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Decode failed: {}", e);
            return JObject::null();
        }
    };

    create_result_object(&mut env, result)
}

#[no_mangle]
pub extern "system" fn Java_com_example_colorcode_ColorCodeNative_nativeFree(
    _env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
) {
    if decoder_ptr != 0 {
        unsafe { Box::from_raw(decoder_ptr as *mut Decoder); }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_colorcode_ColorCodeNative_nativeEncode(
    mut env: JNIEnv,
    _class: JClass,
    data: JByteArray,
    ecc_level: jint,
    module_size: jint,
    compression: jint,
) -> JByteArray {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("ColorCodeEncoder"),
    );

    let input_data: Vec<u8> = env.convert_byte_array(&data).unwrap_or_default();

    let ecc = match ecc_level {
        0 => EccLevel::Low,
        1 => EccLevel::Medium,
        2 => EccLevel::High,
        3 => EccLevel::Maximum,
        _ => EccLevel::Low,
    };

    let comp = match compression {
        0 => Compression::None,
        1 => Compression::Lz4,
        2 => Compression::Zstd,
        _ => Compression::None,
    };

    let options = EncoderOptions {
        ecc_level: ecc,
        compression: comp,
        module_size: module_size as u32,
        quiet_zone: 4,
        version: None,
    };

    let code = match Encoder::new(options).encode_bytes(&input_data) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Encoding failed: {}", e);
            return env.byte_array_from_slice(&[]).unwrap();
        }
    };

    let png_data = code
        .matrix
        .render_png_with_quiet_zone(module_size as u32, 4);
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    if let Err(e) = encoder.write_image(
        png_data.as_raw(),
        png_data.width(),
        png_data.height(),
        image::ExtendedColorType::Rgb8,
    ) {
        log::error!("PNG encoding failed: {}", e);
        return env.byte_array_from_slice(&[]).unwrap();
    }

    env.byte_array_from_slice(&png_bytes).unwrap()
}

fn create_result_object(env: &mut JNIEnv, payload: DecodedPayload) -> JObject {
    // Create byte array for decoded data
    let data_array = match env.byte_array_from_slice(&payload.data) {
        Ok(arr) => arr,
        Err(e) => {
            log::error!("Failed to create data array: {}", e);
            return JObject::null();
        }
    };

    // Create Metadata object first
    let metadata_class = match env.find_class(METADATA_CLASS) {
        Ok(cls) => cls,
        Err(e) => {
            log::error!("Failed to find Metadata class: {}", e);
            return JObject::null();
        }
    };

    let metadata_obj = match env.new_object(
        &metadata_class,
        "(IIIII)V",
        &[
            jni::objects::JValue::Int(payload.metadata.version.id() as i32),
            jni::objects::JValue::Int(payload.metadata.ecc_level.id() as i32),
            jni::objects::JValue::Int(payload.metadata.compression.id() as i32),
            jni::objects::JValue::Int(payload.metadata.payload_len as i32),
            jni::objects::JValue::Int(payload.ecc_corrections as i32),
        ],
    ) {
        Ok(obj) => obj,
        Err(e) => {
            log::error!("Failed to create Metadata object: {}", e);
            return JObject::null();
        }
    };

    let crc_valid = payload.crc32_valid;

    // Find the DecodeResult class
    let result_class = match env.find_class(RESULT_CLASS) {
        Ok(cls) => cls,
        Err(e) => {
            log::error!("Failed to find DecodeResult class: {}", e);
            return JObject::null();
        }
    };

    // Create DecodeResult object with constructor (ByteArray, Metadata, boolean)
    match env.new_object(
        &result_class,
        "([BLcom/example/colorcode/Metadata;Z)V",
        &[
            jni::objects::JValue::Object(&data_array),
            jni::objects::JValue::Object(&metadata_obj),
            jni::objects::JValue::Bool(crc_valid as u8),
        ],
    ) {
        Ok(obj) => obj,
        Err(e) => {
            log::error!("Failed to create DecodeResult object: {}", e);
            JObject::null()
        }
    }
}