extern crate flate2;
extern crate yaml_rust2;
extern crate tar;
extern crate fs_extra;
extern crate sha2;
extern crate indicatif;
extern crate pretty_env_logger;

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
use crate::package::utils::{add_package, check_exist_pkg, PackageManifest, script_executor};
use crate::repo::utils::{get_repos, search_pkg, fetch_url, find_package_by_version};
use crate::package::depencies::PackageQuery;
use crate::consts::paths::{TMP_PATH, DB_PATH, REPOS_FILE};
use walkdir::WalkDir;
use log::{info, error};
use indicatif::{ProgressBar, ProgressStyle};
use pretty_env_logger::formatted_builder;
use std::pin::Pin;
use futures::Future;

fn setup_logger() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        formatted_builder()
            .filter(None, log::LevelFilter::Info)
            .init();
    });
}

pub fn parse_manifest(path: &Path) -> Result<PackageManifest, Box<dyn Error>> {
    let manifest_path = path.join("package.yml");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let docs = YamlLoader::load_from_str(&content)
        .map_err(|e| format!("Invalid YAML syntax: {}", e))?;
    let root = docs.first().ok_or("Empty YAML document")?;
    let name = root["name"].as_str().ok_or("Missing required field 'name'")?.to_string();
    let version = root["version"].as_str().ok_or("Missing required field 'version'")?.to_string();
    let depens = root["depens"].as_vec()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_else(Vec::new);
    Ok(PackageManifest { name, version, depens })
}

// Функция для подтверждения установки пакета
fn confirm_installation(manifest: &PackageManifest) -> bool {
    println!("Пакет: {} версии {}", manifest.name, manifest.version);
    if (!manifest.depens.is_empty()) {
        println!("Необходимые зависимости:");
        for dep in &manifest.depens {
            println!("- {}", dep);
        }
    }
    print!("Вы уверены, что хотите установить этот пакет? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn mask_copyer(path: &Path) -> Result<(), Box<dyn Error>> {
    let src = path.join("mask");
    let dst = Path::new("/");
    let options = CopyOptions::new().overwrite(true).content_only(true).copy_inside(true);
    copy(src, dst, &options)?;
    Ok(())
}

fn create_package_dir(manifest: &PackageManifest) -> Result<PathBuf, Box<dyn Error>> {
    let dir_name = format!("{}-{}", manifest.name, manifest.version);
    let path = PathBuf::from("/var/lib/konpac/packages").join(&dir_name);
    fs::create_dir_all(&path)?;
    if (!path.exists()) {
        return Err(format!("Failed to create directory: {:?}", path).into());
    }
    Ok(path)
}

fn hash_package(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];
    let pb = ProgressBar::new(file.metadata()?.len());
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if (bytes_read == 0) {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        pb.inc(bytes_read as u64);
    }
    pb.finish_with_message("Hashing complete");
    let hash = hasher.finalize();
    Ok(format!("{:02x}", hash))
}

fn unpack_package(path: &Path, out: &Path) -> Result<(), io::Error> {
    let package = File::open(path)?;
    let unzip_pkg = GzDecoder::new(package);
    let mut archive = Archive::new(unzip_pkg);
    archive.unpack(out)?;
    Ok(())
}

fn copy_scripts(temp_pkg: &Path, package_dir: &Path) -> Result<(), io::Error> {
    let scripts_path = temp_pkg.join("scripts");
    let options = CopyOptions::new().overwrite(true).content_only(true).copy_inside(true);
    let dest = package_dir.join("scripts");
    copy(scripts_path, dest, &options).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

fn create_package_list(mask_root: &Path, package_dir: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(package_dir)?;
    if (!mask_root.exists()) {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Mask directory '{}' not found", mask_root.display())));
    }
    let list_path = package_dir.join("package.list");
    let mut file = File::create(&list_path)?;
    for entry in WalkDir::new(mask_root).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        let rel_path = entry.path().strip_prefix(mask_root).unwrap().to_string_lossy();
        writeln!(file, "/{}", rel_path)?;
    }
    Ok(())
}

pub async fn install_package_from_file(path: &Path, yes: bool) -> Result<(), Box<dyn Error>> {
    info!("Хеширование пакета: Подготовка");
    let hash = hash_package(path)?;
    info!("Хеширование пакета завершено");

    info!("Распаковка пакета: Подготовка");
    let tmpdir = TMP_PATH.to_string();
    let temp_package_path_str = format!("{}/{}", tmpdir, hash);
    let temp_package_path = Path::new(&temp_package_path_str);
    let output_path_str = format!("{}/{}", tmpdir, hash);
    let output_path = Path::new(&output_path_str);
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(100);
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .tick_strings(&["|", "/", "-", "\\"]));
    pb.set_message("Распаковка пакета...");
    unpack_package(path, output_path)?;
    pb.finish_with_message("Распаковка завершена");
    info!("Распаковка пакета завершена");

    info!("Чтение манифеста: Подготовка");
    let package = parse_manifest(temp_package_path)?;
    info!("Чтение манифеста завершено");

    // Подтверждение установки пакета
    if !yes && !confirm_installation(&package) {
        info!("Установка пакета отменена");
        return Ok(());
    }

    info!("Проверка существования пакета: Проверка");
    let db_path = Path::new(DB_PATH);
    if check_exist_pkg(db_path, &package.name)? {
        info!("Пакет уже установлен");
        return Ok(());
    }
    info!("Проверка существования пакета завершена");

    info!("Установка зависимостей: Установка");
    for depen in &package.depens {
        let depency = PackageQuery::parse(depen)?;
        if check_exist_pkg(db_path, &depency.name)? {
            info!("Зависимость уже установлена: {}", depency.name);
            continue;
        }
        let repositories = get_repos(Path::new("/etc/konpac/repos"));
        let mut found_package = None;
        for repo in repositories {
            match find_package_by_version(&depency.name, &depency.version, &depency.comparison_operator, repo).await {
                Ok(Some(pkg)) => {
                    found_package = Some(pkg);
                    break;
                }
                Ok(None) => { continue; }
                Err(e) => {
                    error!("Ошибка поиска в репозитории: {}", e);
                    continue;
                }
            }
        }
        let found_package = found_package.ok_or_else(|| format!("Зависимость не найдена ни в одном репозитории: {}", depency.name))?;
        Pin::from(Box::new(install_from_repo(&found_package.name, yes))).await?;
    }
    info!("Установка зависимостей завершена");

    info!("Выполнение скрипта установки: Установка");
    script_executor(temp_package_path, "install");
    info!("Выполнение скрипта установки завершено");

    info!("Копирование файлов маски: Установка");
    mask_copyer(temp_package_path)?;
    info!("Копирование файлов маски завершено");

    info!("Создание директории пакета: Установка");
    let var_package_path = create_package_dir(&package)?;
    info!("Создание директории пакета завершено");

    info!("Копирование скриптов: Установка");
    copy_scripts(temp_package_path, &var_package_path)?;
    info!("Копирование скриптов завершено");

    info!("Создание списка файлов пакета: Установка");
    create_package_list(&temp_package_path.join("mask"), &var_package_path)?;
    info!("Создание списка файлов пакета завершено");

    info!("Добавление пакета в базу данных: Завершение");
    add_package(&package, &var_package_path, db_path)?;
    info!("Добавление пакета в базу данных завершено");

    Ok(())
}

pub async fn install_from_repo(name: &str, yes: bool) -> Result<(), Box<dyn Error>> {
    info!("Проверка существования пакета: Проверка");
    let db_path = Path::new(DB_PATH);
    if check_exist_pkg(db_path, name)? {
        info!("Пакет уже установлен");
        return Ok(());
    }
    info!("Проверка существования пакета завершена");

    info!("Поиск пакета в репозиториях: Поиск");
    let repositories = get_repos(Path::new("/etc/konpac/repos"));
    let mut found_package = None;
    for repo in repositories {
        match search_pkg(name, repo).await {
            Ok(pkg) => {
                found_package = Some(pkg);
                break;
            }
            Err(e) => {
                error!("Ошибка поиска в репозитории: {}", e);
                continue;
            }
        }
    }
    let found_package = found_package.ok_or_else(|| "Пакет не найден ни в одном репозитории")?;
    info!("Поиск пакета в репозиториях завершен");

    info!("Загрузка пакета: Загрузка");
    let package_file_name = format!("/tmp/{}-{}.kpkg", found_package.name, found_package.version);
    let package_file_path = Path::new(&package_file_name);
    fetch_url(found_package.url, package_file_path).await?;
    info!("Загрузка пакета завершена");

    info!("Установка пакета из файла: Установка");
    Pin::from(Box::new(install_package_from_file(package_file_path, yes))).await?;
    info!("Установка пакета из файла завершена");

    info!("Пакет загружен в: {:#?}", package_file_name);
    Ok(())
}
