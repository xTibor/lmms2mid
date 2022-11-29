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

    //#[xml(child = "data")]
    //pub data: SvData,
}

impl LmmsProject {
    pub fn load_compressed(path: &Path) -> Result<Self, Box<dyn Error>> {
        let compressed_bin = fs::read(path)?;
        let uncompressed_bin = decompress_to_vec_zlib(&compressed_bin[4..]).unwrap();
        let uncompressed_xml = str::from_utf8(&uncompressed_bin).unwrap();

        Ok(LmmsProject::from_str(&uncompressed_xml)?)
    }
}
