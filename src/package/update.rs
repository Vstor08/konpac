use std::error::Error;
use crate::package::utils::check_package_local;
use crate::repo::utils::search_pkg;
use std::path::Path;
use crate::consts::paths::DB_PATH;

pub fn update_package(name: &str) -> Result<(), Box<dyn Error>> {
    let package = check_package_local(Path::new(DB_PATH), name)?;
    let package = match package {
        Some(pkg) => pkg,
        None => return Err("Package not found".into()),
    };

    Ok(())
}