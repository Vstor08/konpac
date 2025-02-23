extern crate flate2;
extern crate yaml_rust2;
extern crate tar;
extern crate fs_extra;
extern crate sha2;

use sha2::{Sha256,Digest};
use flate2::read::GzDecoder;
use yaml_rust2::{YamlLoader,YamlEmitter};
use std::fmt::format;
use std::path::Path;
// use std::collections::BTreeMap;
// use std::fmt::format;
use std::{fs, path};
use std::fs::{OpenOptions,File};
use tar::Archive;
use std::process::Command;
use std::io::{Read,Write};
use fs_extra::dir::{copy,CopyOptions};
// use std::path::Path;

fn remove_all_extensions(path_name: String) -> String {
    let path = Path::new(&path_name);
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.split('.').next().unwrap_or(name))
        .unwrap_or_default()
        .to_string()
}


fn parse_manifest(path: String) -> String {
    println!("{}",path);
    let manifest_path = format!("{}/package.yml",path);
    println!("{}",manifest_path);
    let manifest_text = fs::read_to_string(manifest_path).expect("error in read manifest");
    let manifest = YamlLoader::load_from_str(&manifest_text).unwrap();
    let name_package = format!("{}",&manifest[0]["name"].as_str().unwrap());
    return name_package;    

}



fn install_script_executor(path: String) {
    let script_path = format!("{}/install",path);
    //let mask_path = format!("{}/mask",path);
    let src_path = format!("{}/src",path);
    let mask_path = format!("{}/mask",path);
    let output = Command::new("bash").arg(script_path).arg(src_path).arg(mask_path).output().unwrap();
    println!("{:#?}",output)

}

 fn mask_copyer(path: String) -> Result<(), Box<dyn std::error::Error>> {
    let src = Path::new(&path).join("mask");
    let dst = Path::new("/");

    let options = CopyOptions::new()
        .overwrite(true)
        .content_only(true)
        .copy_inside(true);

    copy(src, dst, &options)?;
    Ok(())

} 


fn hash_package(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(format!("{}",path))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    // Читаем файл по частям и обновляем хеш
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:02x}",hash))
}

fn unpack_package(path: &str,out: &str) -> Result<(), std::io::Error> {
    /* Функция распоковки пакета во временную папку */
    let package = File::open(path)?;
    let unzip_pkg = GzDecoder::new(package);
    let mut archive = Archive::new(unzip_pkg);
    let out = format!("{}/{}",out,hash_package(path).unwrap());
    archive.unpack(out)?;

    Ok(())
}

fn write_to_bd(name: String) -> std::io::Result<()>{
    let bd = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/etc/konpac/packages");
    writeln!(bd?, "{}",name)?;
    Ok(())
}

pub fn install_package_from_file(path: &str,isroot: bool) {
    let mut tmpdir = String::new();
    if isroot {
        tmpdir = String::from("/tmp")
    } else {
        panic!("Please restart as root")
    }
    unpack_package(path, &tmpdir).expect("error in unpack package");
    let hash: String = hash_package(path).unwrap();
    let name_package = parse_manifest(format!("/tmp/{}/{}",hash,remove_all_extensions(path.to_string())));
    install_script_executor(format!("{}/{}/{}",tmpdir,hash,remove_all_extensions(path.to_string())));
    mask_copyer(format!("{}/{}/{}",tmpdir,hash,remove_all_extensions(path.to_string()))).expect("sex");
    write_to_bd(name_package);
}