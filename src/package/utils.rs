extern crate libc;
extern crate rusqlite;

use rusqlite::{Connection, Result};
use std::path::Path;

struct Package {
    name: String,
    version: String,
    path: Path
}

pub fn is_elevated() -> bool {
    unsafe { libc::getuid() == 0 }
}



pub fn add_package(package: &Package) -> Result<()> {

    // TODO: create add to database functional

    Ok(())
}

pub fn del_package(name: String) -> Result<()> {
    // TODO: create delete from database functional
    Ok(())
}