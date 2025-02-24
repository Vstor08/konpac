
use std::{fs::{self, File}, path::{Path,PathBuf}};
use walkdir::WalkDir;
use rusqlite::{params, Connection, Params, Result};
use crate::repo::utils::DB_Package_Entry;


fn write_repo_db(package: DB_Package_Entry,db_path: &Path) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS packages (
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            path TEXT NOT NULL,
            PRIMARY KEY (name, version)
        )", 
        [],
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO packages (name, version, path)
         VALUES (?1, ?2, ?3)",
        params![
            package.name,
            package.version,
            package.url,
        ],
    )?;

    Ok(())
}

pub fn generate_repo(path: PathBuf) {
    let db_path = path.join("packages.db");
    let mut file = File::create(&db_path);
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "kpkg")) {
            let version = entry.file_name() // Получаем имя файла
                .to_str() // Преобразуем в &str
                .unwrap_or("") // Если преобразование не удалось, используем пустую строку
                .rsplitn(2, '-') // Разделяем строку по последнему тире (максимум 2 части)
                .next() // Берем часть после последнего тире
                .unwrap_or("") // Если разделение не удалось, используем пустую строку
                .rsplitn(2, '.') // Разделяем по последней точке (максимум 2 части)
                .nth(1) // Берем часть до последней точки
                .unwrap_or(""); 
                 
            let prefix = entry.file_name() // Получаем имя файла
                .to_str() // Преобразуем в &str
                .unwrap_or("") // Если преобразование не удалось, используем пустую строку
                .split('-') // Разделяем строку по тире
                .next() // Берем первую часть (до первого тире)
                .unwrap_or(""); // Если разделение не удалось, используем пустую строку
            let absolute_path = fs::canonicalize(entry.path()).unwrap();
            println!("{:?}", absolute_path);
            let package: DB_Package_Entry = DB_Package_Entry{
                name: prefix.to_string(),
                version: version.to_string(),
                url: format!("file://{}",&absolute_path.to_str().unwrap())
            };
            
            write_repo_db(package, &db_path);
        }
}