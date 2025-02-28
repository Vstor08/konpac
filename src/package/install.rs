// Подключаем внешние библиотеки
extern crate flate2;
extern crate yaml_rust2;
extern crate tar;
extern crate fs_extra;
extern crate sha2;

// Импортируем необходимые модули и функции
use sha2::{Sha256, Digest};
use flate2::read::GzDecoder;
use yaml_rust2::YamlLoader;
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::File;
use tar::Archive;

use std::io::{self, Read, Write};
use std::error::Error;
use fs_extra::dir::{copy, CopyOptions};
use crate::package;
use crate::package::utils::{add_package, check_exist_pkg, check_package_local, DbPackageEntry, PackageManifest,script_executor};
use crate::repo::utils::{get_repos, search_pkg, fetch_url, find_package_by_version};
use crate::package::depencies::PackageQuery;
use crate::consts::paths::{TMP_PATH,DB_PATH,REPOS_FILE};
use walkdir::WalkDir;
use version_compare::Version;

// Функция для парсинга manifest-файла пакета
fn parse_manifest(path: &Path) -> Result<PackageManifest, Box<dyn std::error::Error>> {
    // Формируем путь к manifest-файлу
    println!("{:?}", path);
    let manifest_path = path.join("package.yml");
    println!("{:?}", manifest_path);

    // Читаем содержимое файла
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    // Парсим YAML
    let docs = YamlLoader::load_from_str(&content)
        .map_err(|e| format!("Invalid YAML syntax: {}", e))?;

    let root = docs.first()
        .ok_or("Empty YAML document")?;

    // Извлекаем поле "name"
    let name = root["name"]
        .as_str()
        .ok_or("Missing required field 'name'")?
        .to_string();

    // Извлекаем поле "version"
    let version = root["version"]
        .as_str()
        .ok_or("Missing required field 'version'")?
        .to_string();

    // Извлекаем поле "depens" (если оно есть)
    let depens = root["depens"]
        .as_vec()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(Vec::new);

    // Возвращаем структуру PackageManifest
    Ok(PackageManifest {
        name,
        version,
        depens,
    })
}



// Функция для копирования маски в корневую директорию
fn mask_copyer(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let src = Path::new(&path).join("mask");
    let dst = Path::new("/");

    // Настройки копирования
    let options = CopyOptions::new()
        .overwrite(true)
        .content_only(true)
        .copy_inside(true);

    // Копируем маску
    copy(src, dst, &options)?;
    Ok(())
}

// Функция для создания директории пакета
fn create_package_dir(manifest: &PackageManifest) -> Result<PathBuf, Box<dyn Error>> {
    // Формируем имя директории пакета
    let dir_name = format!("{}-{}", manifest.name, manifest.version);
    let mut path = PathBuf::from("/var/lib/konpac/packages");
    path.push(&dir_name);

    // Создаем директорию рекурсивно
    fs::create_dir_all(&path)?;

    // Проверяем, что директория создана
    if !path.exists() {
        return Err(format!("Failed to create directory: {:?}", path).into());
    }

    Ok(path)
}

// Функция для вычисления хеша пакета
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

    // Получаем финальный хеш
    let hash = hasher.finalize();
    Ok(format!("{:02x}", hash))
}

// Функция для распаковки пакета
fn unpack_package(path: &Path, out: &Path) -> Result<(), std::io::Error> {
    // Открываем файл пакета
    let package = File::open(path)?;
    let unzip_pkg = GzDecoder::new(package);
    let mut archive = Archive::new(unzip_pkg);

    // Распаковываем архив
    archive.unpack(out)?;
    Ok(())
}

// Функция для создания списка файлов пакета
fn create_package_list(
    mask_root: &Path,
    package_dir: &Path,
) -> Result<(), std::io::Error> {
    // Создаем целевую директорию
    fs::create_dir_all(package_dir)?;

    // Проверяем существование исходной маски
    if !mask_root.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Директория с маской '{}' не найдена", mask_root.display()),
        ));
    }

    // Создаем файл package.list
    let list_path = package_dir.join("package.list");
    let mut file = File::create(&list_path)?;

    // Рекурсивно обходим файлы в маске
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

        // Записываем путь в файл
        writeln!(file, "/{}", rel_path)?;
    }

    Ok(())
}

// Функция для установки пакета из файла
pub async fn install_package_from_file(path: &Path) -> Result<(), Box<dyn Error>> {
    let tmpdir = String::from(TMP_PATH);
    let db_path: &Path = Path::new(DB_PATH);
    let hash: String = hash_package(&path).unwrap();
    let package_path: &Path = Path::new(&path);
    let temp_path_str = format!("{}/{}", tmpdir, hash);
    let temp_package_path: &Path = Path::new(&temp_path_str);
    let output_path_str = format!("{}/{}", tmpdir, hash);
    let output_path: &Path = Path::new(&output_path_str);
    // let repos_file: &Path = Path::new(REPOS_FILE);
    match unpack_package(package_path, output_path) {
        Ok(_) => println!("Unpacking Success"),
        Err(e) => eprintln!("Error in unpack: {}", e),
    };
    let package = match parse_manifest(&temp_package_path) {
        Ok(manifest) => manifest,
        Err(e) => {
            eprintln!("Error: {}", e);
            panic!("Error in read manifest");
        }
    };
    if check_exist_pkg(db_path, &package.name)? {
        println!("Пакет уже установлен");
        return Ok(());
    }
    for depen in &package.depens {
        let depency: PackageQuery = PackageQuery::parse(&depen)?;
    
        // Проверяем, установлена ли зависимость
        if check_exist_pkg(db_path, &depency.name)? {
            println!("Зависимость уже установлена: {}", depency.name);
            continue;
        }
    
        // Получаем список репозиториев
        let repositories = get_repos(Path::new("/etc/konpac/repos"));
    
        // Ищем пакет в репозиториях
        let mut package = None;
        for repo in repositories {
            match find_package_by_version(&depency.name, &depency.version, &depency.comparison_operator, repo).await {
                Ok(Some(pkg)) => {
                    package = Some(pkg);
                    break; // Выходим из цикла, если пакет найден
                }
                Ok(None) => {
                    // Пакет не найден в этом репозитории, продолжаем поиск
                    continue;
                }
                Err(e) => {
                    eprintln!("Error searching in repository: {}", e);
                    continue; // Продолжаем поиск в следующем репозитории
                }
            }
        }
    
        // Проверяем, был ли найден пакет
        let package = match package {
            Some(pkg) => pkg, // Пакет найден
            None => {
                println!("Dependency not found in any repository: {}", depency.name);
                panic!()
            }
        };
    
        // Устанавливаем пакет с использованием Box::pin
        match Box::pin(install_from_repo(&package.name)).await {
            Ok(_) => println!("Dependency installed successfully: {}", package.name),
            Err(e) => {
                eprintln!("Error installing dependency {}: {}", package.name, e);
                continue; // Пропускаем эту зависимость
            }
        };
        
    }
    script_executor(&temp_package_path);
    mask_copyer(&temp_package_path).expect("Failed to copy mask");
    let var_package_path = match create_package_dir(&package) {
        Ok(path) => path,
        Err(_) => panic!("Error in create package path"),
    };
    println!("{:?}", var_package_path);
    match create_package_list(&temp_package_path.join("mask"), &var_package_path) {
        Ok(_) => { println!("Package list created succes") },
        Err(e) => { eprintln!("Error in creating package list: {}",e); panic!() }
    };
    match add_package(&package, &var_package_path, &db_path) {
        Ok(_) => println!("Database write Success"),
        Err(e) => eprintln!("Error in write Data to db: {}", e),
    };
    Ok(())
}

// Функция для установки пакета из репозитория
pub async fn install_from_repo(name: &str) -> Result<(), Box<dyn Error>> {
    let db_path: &Path = Path::new(DB_PATH);
    if check_exist_pkg(db_path, &name).unwrap() {
        println!("Пакет уже установлен");
        return Ok(());
    }
    // Получаем список репозиториев
    let repositories = get_repos(Path::new("/etc/konpac/repos"));

    // Ищем пакет в репозиториях
    let mut package = None;
    for repo in repositories {
        match search_pkg(name, repo).await {
            Ok(pkg) => {
                package = Some(pkg);
                break; // Выходим из цикла, если пакет найден
            }
            Err(e) => {
                eprintln!("Error searching in repository: {}", e);
                continue; // Продолжаем поиск в следующем репозитории
            }
        }
    }
    println!("{:?}", package);

    // Если пакет не найден, возвращаем ошибку
    let package = match package {
        Some(pkg) => pkg,
        None => return Err("Package not found in any repository".into()),
    };

    // Формируем имя файла для сохранения
    let package_file_name = format!("/tmp/{}-{}.kpkg", package.name, package.version);
    let package_file_path = Path::new(&package_file_name);

    // Загружаем пакет
    fetch_url(package.url, &package_file_path).await?;
    println!("{:?}", package_file_name);

    // Устанавливаем пакет
    match install_package_from_file(&package_file_path).await {
        Ok(_) => println!("Installed Success"),
        Err(e) => eprintln!("Error in installation: {}",e)
    };
    println!("Package downloaded to: {:#?}", package_file_name);

    Ok(())
}