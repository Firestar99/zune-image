use std::fs::read;
use std::path::Path;

fn open_and_read<P: AsRef<Path>>(path: P) -> Vec<u8>
{
    read(path).unwrap()
}

fn decode_ref(data: &[u8]) -> Vec<u8>
{
    let transformations = png::Transformations::EXPAND;

    let mut decoder = png::Decoder::new(data);
    decoder.set_transformations(transformations);
    let mut reader = decoder.read_info().unwrap();

    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    let _ = reader.next_frame(&mut buf).unwrap();

    buf
}

fn decode_zune(data: &[u8]) -> Vec<u8>
{
    zune_png::PngDecoder::new(data).decode_raw().unwrap()
}

fn test_decoding<P: AsRef<Path>>(path: P)
{
    let contents = open_and_read(path);

    let zune_results = decode_zune(&contents);
    let ref_results = decode_ref(&contents);
    assert_eq!(&zune_results, &ref_results);
}

#[test]
fn test_1bpp_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn0g01.png";

    test_decoding(path);
}

#[test]
fn test_2bpp_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn0g02.png";

    test_decoding(path);
}

#[test]
fn test_4bpp_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn0g04.png";

    test_decoding(path);
}

#[test]
fn test_8bpp_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn0g08.png";

    test_decoding(path);
}

#[test]
fn test_16bpp_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn0g16.png";

    test_decoding(path);
}

#[test]
fn test_8bpp_luma_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn2c08.png";

    test_decoding(path);
}

#[test]
fn test_16bpp_luma_basic()
{
    let path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/png_suite/basn2c16.png";

    test_decoding(path);
}