extern crate reqwest;
use std::env::consts::ARCH;
use std::io::Cursor;
use std::{fs,io,path::Path,result::Result};
use ini::Ini;
use rusqlite::{Connection, Row};
use url::Url;

//type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub struct Repository {
    name: String,
    url: String
}

#[derive(Debug)]
pub struct DbPackageEntry {
    pub name: String,
    pub version: String,
    pub url: String
}
impl DbPackageEntry {
    fn from_row(row: &Row) -> Result<Self,Box<dyn std::error::Error>> {
        Ok(DbPackageEntry {
            name: row.get(0)?,
            version: row.get(1)?,
            url: row.get(2)?
        })
    }
}

pub fn get_repos(config_file: &Path) -> Vec<Repository> {
    let repos = Ini::load_from_file(config_file).unwrap();
    let mut repositories: Vec<Repository> = vec![];
    for i in repos {
        let repo_name = i.0.unwrap_or("".to_string());
        let repo_url = i.1.get("url").unwrap_or("").replace("$repo", &repo_name).replace("$arch", ARCH);
        let repo: Repository = Repository{
            name: repo_name,
            url: repo_url,
        };
        repositories.push(repo);
    }
    return repositories;
}

pub async fn fetch_url(url: String, file_name: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_url = Url::parse(&url)?;

    match parsed_url.scheme() {
        "http" | "https" => {
            // Обработка HTTP/HTTPS ссылок
            let response = reqwest::get(url).await?;
            let mut file = fs::File::create(Path::new(file_name))?;
            let mut content = Cursor::new(response.bytes().await?);
            io::copy(&mut content, &mut file)?;
        }
        "file" => {
            // Обработка файловых ссылок (file:///)
            let path = Path::new(parsed_url.path());
            let mut source_file = fs::File::open(path)?;
            let mut destination_file = fs::File::create(Path::new(file_name))?;
            io::copy(&mut source_file, &mut destination_file)?;
        }
        _ => return Err("Unsupported URL scheme".into()),
    }

    Ok(())
}

fn find_package(db_path: &Path, package_name: &str) -> Result<Option<DbPackageEntry>,Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;
    
    // Меняем запрос на выбор всех полей
    let mut stmt = conn.prepare("SELECT name, version, path FROM packages WHERE name = ?1")?;
    let mut rows = stmt.query([package_name])?;

    if let Some(row) = rows.next()? {
        // Используем наш метод преобразования
        Ok(Some(DbPackageEntry::from_row(&row)?))
    } else {
        Ok(None)
    }
}

pub async fn search_pkg(pkg_name: String, repo: Repository) -> std::result::Result<DbPackageEntry, Box<dyn std::error::Error>> {
    let db_str_path = format!("/tmp/{}.db", repo.name);
    let db_path = Path::new(&db_str_path);
    
    let db_link = format!("{}/packages.db",repo.url);

    // Загружаем базу данных репозитория
    match fetch_url(db_link, &db_path).await {
        Ok(_) => { println!("fetch file success") },
        Err(e) => { eprintln!("Error in fetch file: {}",e)} 
    };

    // Ищем пакет в базе данных
    let package = find_package(&db_path, &pkg_name)?;
    
    // Обрабатываем результат поиска
    let package = match package {
        Some(pkg) => pkg,
        None => {
            eprintln!("Пакет '{}' не найден в репозитории {}", pkg_name, repo.name);
            panic!()
        }
    };


    Ok(package)
}
 