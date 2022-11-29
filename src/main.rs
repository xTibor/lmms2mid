use miniz_oxide::inflate::decompress_to_vec_zlib;
use std::str;

fn main() {
    let test_bin = include_bytes!("../test/test.mmpz");

    let decompress_result = decompress_to_vec_zlib(&test_bin[4..]);
    println!("{}", str::from_utf8(&decompress_result.unwrap()).unwrap());
}
