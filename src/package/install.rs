extern crate flate2;
extern crate yaml_rust2;
extern crate tar;
extern crate fs_extra;
extern crate sha2;



use sha2::{Sha256,Digest};
use flate2::read::GzDecoder;
use yaml_rust2::YamlLoader;
use std::path::{Path,PathBuf};
use std::fs;
use std::fs::{OpenOptions,File};
use tar::Archive;
use std::process::Command;
use std::io::{self,Read,Write};
use std::error::Error;
use fs_extra::dir::{copy,CopyOptions};
use crate::package::utils::{PackageManifest,add_package};
use walkdir::WalkDir;


fn remove_all_extensions(path_name: String) -> String {
    let path = Path::new(&path_name);
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.split('.').next().unwrap_or(name))
        .unwrap_or_default()
        .to_string()
}


fn parse_manifest(path: &Path) -> Result<PackageManifest, Box<dyn std::error::Error>> {
    // Формируем путь к manifest-файлу
    println!("{:?}",path);
    let manifest_path = path.join("package.yml");
    println!("{:?}",manifest_path);    
    // Читаем файл
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    
    // Парсим YAML
    let docs = YamlLoader::load_from_str(&content)
        .map_err(|e| format!("Invalid YAML syntax: {}", e))?;
    
    let root = docs.first()
        .ok_or("Empty YAML document")?;

    // Извлекаем обязательные поля
    Ok(PackageManifest {
        name: root["name"]
            .as_str()
            .ok_or("Missing required field 'name'")?
            .to_string(),
            
        version: root["version"]
            .as_str()
            .ok_or("Missing required field 'version'")?
            .to_string(),
    })
}



fn install_script_executor(path: &Path) {
    println!("{:?}",path);
    let script_path = path.join("install");
    println!("{:?}",script_path);
    //let mask_path = format!("{}/mask",path);
    let src_path = path.join("src");
    let mask_path = path.join("mask");
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

fn create_package_dir(manifest: &PackageManifest) -> Result<PathBuf, Box<dyn Error>> {
    // Формируем путь к директории пакета
    let dir_name = format!("{}-{}", manifest.name, manifest.version);
    let mut path = PathBuf::from("/var/lib/konpac/packages");
    path.push(&dir_name);

    // Создаем директорию рекурсивно
    fs::create_dir_all(&path)?;
    
    // Проверяем, что директория действительно создалась
    if !path.exists() {
        return Err(format!("Failed to create directory: {:?}", path).into());
    }

    Ok(path)
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



fn create_package_list(
    mask_root: &Path,
    package_dir: &Path,
) -> io::Result<()> {
    // Создаем целевую директорию
    fs::create_dir_all(package_dir)?;

    // Проверка существования исходной маски
    if !mask_root.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Директория с маской '{}' не найдена", mask_root.display()),
        ));
    }

    // Создаем файл package.list
    let list_path = package_dir.join("package.list");
    let mut file = File::create(&list_path)?;

    // Рекурсивный обход файлов
    for entry in WalkDir::new(mask_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        // Получаем относительный путь
        let rel_path = entry.path()
            .strip_prefix(mask_root)
            .unwrap()
            .to_string_lossy();

        // Записываем в формате /path/to/file
        writeln!(file, "/{}", rel_path)?;
    }

    Ok(())
}

pub fn install_package_from_file(path: &Path,isroot: bool) {
    let mut tmpdir = String::new();
    
    if isroot {
        tmpdir = String::from("/tmp")
    } else {
        panic!("Please restart as root")
    }

    let db_path: &Path = Path::new("/var/lib/konpac/packages.db");
    let hash: String = hash_package(&path).unwrap();
    let package_path: &Path = Path::new(&path);
    let package_file_name: String = remove_all_extensions(package_path.to_str().unwrap().to_string());
    let temp_path_str = format!("{}/{}/{}", tmpdir, hash, remove_all_extensions(package_file_name));
    let temp_package_path: &Path = Path::new(&temp_path_str);
    let output_path_str = format!("{}/{}",tmpdir,hash);
    let output_path: &Path = Path::new(&output_path_str);
    

    match unpack_package(package_path, output_path) {
        Ok(a) => { println!("Unpacking Succes") },
        Err(e) => {eprintln!("Error in unpack: {}",e)}
    };
    let package = match parse_manifest(&temp_package_path) {
        Ok(manifest) => manifest,
        Err(e) => {
            eprintln!("Error: {}", e);
            panic!("Error in read manifest")
        },
    };
    install_script_executor(&temp_package_path);
    mask_copyer(&temp_package_path).expect("sex");
    let var_pacakge_path = match create_package_dir(&package) {
        Ok(path) => path,
        Err(_) => panic!("Error in create package path")
    };
    println!("{:?}",var_pacakge_path);
    create_package_list(&temp_package_path.join("/mask"), &var_pacakge_path);
    match add_package(&package,&var_pacakge_path,&db_path) {
        Ok(a) => { println!("Database write Succes") },
        Err(e) => { eprintln!("Error in write Data to db: {}",e) }
    };
}