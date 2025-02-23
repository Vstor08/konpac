extern crate flate2;
extern crate yaml_rust2;
extern crate tar;
extern crate fs_extra;
extern crate sha2;

use sha2::{Sha256,Digest};
use flate2::read::GzDecoder;
use yaml_rust2::YamlLoader;
use std::path::Path;
use std::fs;
use std::fs::{OpenOptions,File};
use tar::Archive;
use std::process::Command;
use std::io::{Read,Write};
use fs_extra::dir::{copy,CopyOptions};


fn remove_all_extensions(path_name: String) -> String {
    let path = Path::new(&path_name);
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.split('.').next().unwrap_or(name))
        .unwrap_or_default()
        .to_string()
}


fn parse_manifest(path: &Path) -> String {
    let manifest_path: &Path = &path.join(Path::new("/package.yml"));
    let manifest_text = fs::read_to_string(manifest_path).expect("error in read manifest");
    let manifest = YamlLoader::load_from_str(&manifest_text).unwrap();
    let name_package = format!("{}",&manifest[0]["name"].as_str().unwrap());
    return name_package;    

}



fn install_script_executor(path: &Path) {
    let script_path = &path.join(Path::new("/install"));
    //let mask_path = format!("{}/mask",path);
    let src_path = &path.join(Path::new("/src"));
    let mask_path = &path.join(Path::new("/mask"));
    let output = Command::new("bash").arg(script_path).arg(src_path).arg(mask_path).output().unwrap();
    println!("{:#?}",output)

}

 fn mask_copyer(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let src = Path::new(&path).join("mask");
    let dst = Path::new("/");

    let options = CopyOptions::new()
        .overwrite(true)
        .content_only(true)
        .copy_inside(true);

    copy(src, dst, &options)?;
    Ok(())

} 


fn hash_package(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
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

fn unpack_package(path: &Path,out: &Path) -> Result<(), std::io::Error> {
    /* Функция распоковки пакета во временную папку */
    let package = File::open(path)?;
    let unzip_pkg = GzDecoder::new(package);
    let mut archive = Archive::new(unzip_pkg);
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

fn create_files_list(path: &Path) {

}

pub fn install_package_from_file(path: &Path,isroot: bool) {
    let mut tmpdir = String::new();
    
    if isroot {
        tmpdir = String::from("/tmp")
    } else {
        panic!("Please restart as root")
    }

    let hash: String = hash_package(&path).unwrap();
    let package_path: &Path = Path::new(&path);
    let package_file_name: String = remove_all_extensions(package_path.to_str().unwrap().to_string());
    let temp_path_str = format!("/{}/{}/{}", tmpdir, hash, remove_all_extensions(package_file_name));
    let temp_package_path: &Path = Path::new(&temp_path_str);
    let output_path_str = format!("/{}/{}/",tmpdir,hash);
    let output_path: &Path = Path::new(&output_path_str);
    let name_package = parse_manifest(&temp_package_path);

    unpack_package(package_path, output_path).expect("error in unpack package");
    install_script_executor(&temp_package_path);
    mask_copyer(&temp_package_path).expect("sex");
    let _ = write_to_bd(name_package);
}