use std::error::Error;
use std::path::Path;
use std::{fs, str};

use miniz_oxide::inflate::decompress_to_vec_zlib;
use strong_xml::XmlRead;

#[derive(Debug, XmlRead)]
#[xml(tag = "lmms-project")]
pub struct LmmsProject {
    #[xml(attr = "type")]
    pub r#type: String,

    #[xml(attr = "version")]
    pub version: usize,

    #[xml(attr = "creator")]
    pub creator: String,

    #[xml(attr = "creatorversion")]
    pub creator_version: String,

    #[xml(child = "head")]
    pub head: LmmsHead,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "head")]
pub struct LmmsHead {
    #[xml(attr = "timesig_denominator")]
    pub time_signature_denominator: usize,

    #[xml(attr = "timesig_numerator")]
    pub time_signature_numerator: usize,

    #[xml(attr = "bpm")]
    pub bpm: usize,

    #[xml(attr = "masterpitch")]
    pub master_pitch: isize,

    #[xml(attr = "mastervol")]
    pub master_volume: usize,
}

impl LmmsProject {
    pub fn load_compressed(path: &Path) -> Result<Self, Box<dyn Error>> {
        let compressed_bin = fs::read(path)?;
        let uncompressed_bin = decompress_to_vec_zlib(&compressed_bin[4..])?;
        let uncompressed_xml = str::from_utf8(&uncompressed_bin)?;

        Ok(LmmsProject::from_str(&uncompressed_xml)?)
    }
}
