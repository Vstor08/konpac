extern crate flate2;
extern crate serde_yaml;
extern crate tar;

use flate2::read::GzDecoder;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use tar::Archive;

pub fn install_package_from_file(path: &str) {
    let manifest_map = BTreeMap::new();
    let manifest_path = format!("{}/package.yml", path);
    let manifest_text = fs::read_to_string(manifest_path).expect("error in read manifest");
    let manifest: BTreeMap<String, String> = serde_yaml::from_str(&manifest_text).unwrap();
    assert_eq!(manifest_map, manifest);
}

fn unpack_package(path: &str, out: &str) -> Result<(), std::io::Error> {
    let package = File::open(path)?;
    let unzip_pkg = GzDecoder::new(package);
    let mut archive = Archive::new(unzip_pkg);
    archive.unpack(out)?;

    Ok(())
}
