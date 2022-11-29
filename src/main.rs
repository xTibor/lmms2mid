use std::path::Path;

mod lmms_model;
use lmms_model::LmmsProject;

fn main() {
    let lmms_project = LmmsProject::load_compressed(Path::new("test/test.mmpz"));
    println!("{:#?}", lmms_project);
}
