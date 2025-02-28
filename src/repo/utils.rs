// Внешние зависимости
extern crate reqwest;
use std::env::consts::ARCH;
use std::io::Cursor;
use std::{fs,io,path::Path,result::Result};
use ini::Ini;
use rusqlite::{Connection, Row};
use url::Url;
use version_compare::Version;
use log::{info, error};

// Структура для хранения информации о репозитории
pub struct Repository {
    name: String, // Имя репозитория
    url: String   // URL репозитория
}

// Структура для хранения информации о пакете из базы данных
#[derive(Debug)]
pub struct DbPackageEntry {
    pub name: String,    // Имя пакета
    pub version: String, // Версия пакета
    pub url: String     // URL для скачивания пакета
}

// Реализация создания DbPackageEntry из строки базы данных
impl DbPackageEntry {
    fn from_row(row: &Row) -> Result<Self,Box<dyn std::error::Error>> {
        Ok(DbPackageEntry {
            name: row.get(0)?,
            version: row.get(1)?,
            url: row.get(2)?
        })
    }
}

// Получение списка репозиториев из конфигурационного файла
pub fn get_repos(config_file: &Path) -> Vec<Repository> {
    let repos = Ini::load_from_file(config_file).unwrap();
    let mut repositories: Vec<Repository> = vec![];
    
    // Обработка каждой секции в INI файле
    for i in repos {
        let repo_name = i.0.unwrap_or("".to_string());
        // Замена переменных в URL репозитория
        let repo_url = i.1.get("url")
            .unwrap_or("")
            .replace("$repo", &repo_name)
            .replace("$arch", ARCH);
            
        let repo = Repository{
            name: repo_name,
            url: repo_url,
        };
        repositories.push(repo);
    }
    return repositories;
}




// Загрузка файла по URL
pub async fn fetch_url(url: String, file_name: &Path) -> Result<(), Box<dyn std::error::Error>> {
    info!("Fetching URL: {}", url);
    let parsed_url = Url::parse(&url)?;

    match parsed_url.scheme() {
        "http" | "https" => {
            // Загрузка файла по HTTP/HTTPS
            let response = reqwest::get(url).await?;
            let mut file = fs::File::create(file_name)?;
            let mut content = Cursor::new(response.bytes().await?);
            io::copy(&mut content, &mut file)?;
        }
        "file" => {
            // Копирование локального файла
            let path = Path::new(parsed_url.path());
            let mut source_file = fs::File::open(path)?;
            let mut destination_file = fs::File::create(file_name)?;
            io::copy(&mut source_file, &mut destination_file)?;
        }
        _ => return Err("Unsupported URL scheme".into()),
    }

    Ok(())
}

// Поиск последней версии пакета в базе данных
fn find_package(
    db_path: &Path,
    package_name: &str,
) -> Result<Option<DbPackageEntry>, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

    // SQL запрос для поиска последней версии пакета
    let mut stmt = conn.prepare(
        "SELECT name, version, path FROM packages WHERE name = ?1 ORDER BY version DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([package_name])?;

    if let Some(row) = rows.next()? {
        Ok(Some(DbPackageEntry::from_row(&row)?))
    } else {
        Ok(None)
    }
}

// Поиск пакета с учетом минимальной версии
fn find_package_with_ver(
    db_path: &Path,
    package_name: &str,
    version: &str,
) -> Result<Option<DbPackageEntry>, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare("SELECT name, version, path FROM packages WHERE name = ?1")?;
    let mut rows = stmt.query([package_name])?;

    let min_version = Version::from(version).ok_or("Invalid version format")?;
    let mut latest_entry: Option<DbPackageEntry> = None;

    // Перебор всех версий пакета
    while let Some(row) = rows.next()? {
        let entry = DbPackageEntry::from_row(&row)?;
        let entry_version = Version::from(&entry.version).ok_or("Invalid version format in database")?;

        // Проверка версии и обновление latest_entry при необходимости
        if entry_version >= min_version {
            if let Some(ref latest) = latest_entry {
                let latest_version = Version::from(&latest.version).ok_or("Invalid version format in database")?;
                if entry_version > latest_version {
                    latest_entry = Some(entry);
                }
            } else {
                latest_entry = Some(entry);
            }
        }
    }

    Ok(latest_entry)
}

// Поиск пакета по версии с использованием оператора сравнения
pub async fn find_package_by_version(
    package_name: &str,
    version: &str,
    comparison_operator: &str,
    repo: Repository
) -> Result<Option<DbPackageEntry>, Box<dyn std::error::Error>> {
    let db_str_path = format!("/tmp/{}.db", repo.name);
    let db_path = Path::new(&db_str_path);
    
    // Загрузка базы данных репозитория
    let db_link = format!("{}/packages.db", repo.url);
    info!("Fetching database from URL: {}", db_link);
    match fetch_url(db_link, &db_path).await {
        Ok(_) => { info!("Fetch file success") },
        Err(e) => { 
            error!("Error in fetch file: {}", e); 
            return Err(e); 
        }
    };

    let conn = Connection::open(db_path)?;

    // Проверка существования таблицы packages
    conn.execute(
        "CREATE TABLE IF NOT EXISTS packages (
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            path TEXT NOT NULL,
            PRIMARY KEY (name, version)
        )",
        [],
    )?;

    // Формирование SQL запроса в зависимости от оператора сравнения
    let query = match comparison_operator {
        "=" => "SELECT name, version, path FROM packages WHERE name = ?1 AND version = ?2",
        "<" => "SELECT name, version, path FROM packages WHERE name = ?1 AND version < ?2",
        ">" => "SELECT name, version, path FROM packages WHERE name = ?1 AND version > ?2",
        "<=" => "SELECT name, version, path FROM packages WHERE name = ?1 AND version <= ?2",
        ">=" => "SELECT name, version, path FROM packages WHERE name = ?1 AND version >= ?2",
        _ => return Err("Неподдерживаемый оператор сравнения. Используйте =, <, >, <=, >=".into()),
    };

    let mut stmt = conn.prepare(query)?;
    let mut rows = stmt.query([package_name, version])?;

    if let Some(row) = rows.next()? {
        Ok(Some(DbPackageEntry::from_row(&row)?))
    } else {
        Ok(None)
    }
}

// Поиск последней версии пакета в репозитории
pub async fn search_pkg(pkg_name: &str, repo: Repository) -> std::result::Result<DbPackageEntry, Box<dyn std::error::Error>> {
    let db_str_path = format!("/tmp/{}.db", repo.name);
    let db_path = Path::new(&db_str_path);
    
    // Загрузка базы данных репозитория
    let db_link = format!("{}packages.db", repo.url);
    match fetch_url(db_link, &db_path).await {
        Ok(_) => { info!("Fetch file success") },
        Err(e) => { error!("Error in fetch file: {}", e)} 
    };

    info!("Repository URL: {}", repo.url);

    // Поиск пакета
    let package = find_package(&db_path, &pkg_name)?;
    
    // Обработка результата поиска
    let package = match package {
        Some(pkg) => pkg,
        None => {
            error!("Пакет '{}' не найден в репозитории {}", pkg_name, repo.name);
            panic!()
        }
    };

    Ok(package)
}

// Поиск пакета с указанной версией в репозитории
pub async fn search_pkg_with_ver(pkg_name: String, repo: Repository, version: &str) -> std::result::Result<DbPackageEntry, Box<dyn std::error::Error>> {
    let db_str_path = format!("/tmp/{}.db", repo.name);
    let db_path = Path::new(&db_str_path);
    
    // Загрузка базы данных репозитория
    let db_link = format!("{}/packages.db",repo.url);
    match fetch_url(db_link, &db_path).await {
        Ok(_) => { info!("Fetch file success") },
        Err(e) => { error!("Error in fetch file: {}", e)} 
    };

    // Поиск пакета с учетом версии
    let package = find_package_with_ver(&db_path, &pkg_name,version)?;
    
    // Обработка результата поиска
    let package = match package {
        Some(pkg) => pkg,
        None => {
            error!("Пакет '{}' не найден в репозитории {}", pkg_name, repo.name);
            panic!()
        }
    };

    Ok(package)
}