use rusqlite::{Connection, params, Result, Row};
use std::path::{Path, PathBuf};
use std::error::Error;
use std::process::Command;
use log::{info, error};

#[derive(Debug)]
pub struct DbPackageEntry {
    pub name: String,
    pub version: String,
    pub path: String
}
impl DbPackageEntry {
    fn from_row(row: &Row) -> Result<Self,Box<dyn std::error::Error>> {
        Ok(DbPackageEntry {
            name: row.get(0)?,
            version: row.get(1)?,
            path: row.get(2)?
        })
    }
}


#[derive(Debug)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub depens: Vec<String>
}

pub fn is_elevated() -> bool {
    unsafe { libc::getuid() == 0 }
}



pub fn add_package(
    manifest: &PackageManifest,
    package_path: &Path,
    db_path: &Path,
) -> Result<()> {
    // Подключаемся к базе данных
    let conn = Connection::open(db_path)?;
    
    // Создаем таблицу если не существует
    conn.execute(
        "CREATE TABLE IF NOT EXISTS packages (
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            path TEXT NOT NULL,
            PRIMARY KEY (name, version)
        )",
        [],
    )?;

    // Вставляем или обновляем запись
    conn.execute(
        "INSERT OR REPLACE INTO packages (name, version, path)
         VALUES (?1, ?2, ?3)",
        params![
            manifest.name,
            manifest.version,
            package_path.to_string_lossy().to_string()
        ],
    )?;

    Ok(())
}

pub fn del_package(name: String) -> Result<(), Box<dyn Error>> {
    // Открываем соединение с базой данных
    let conn = Connection::open("/var/lib/konpac/packages.db")?;
    
    // Выполняем SQL-запрос на удаление
    let rows_affected = conn.execute(
        "DELETE FROM packages WHERE name = ?1",
        params![name],
    )?;

    // Проверяем, что запись действительно была удалена
    if rows_affected == 0 {
        return Err(format!("Пакет '{}' не найден в базе данных", name).into());
    }

    Ok(())
}

pub fn check_package_local(db_path: &Path,
    package_name: &str,
) -> Result<Option<DbPackageEntry>, Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

    // Ищем последнюю версию пакета
    let mut stmt = conn.prepare(
        "SELECT name, version, path FROM packages WHERE name = ?1 ORDER BY version DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([package_name])?;

    if let Some(row) = rows.next()? {
        Ok(Some(DbPackageEntry::from_row(&row).unwrap()))
    } else {
        Ok(None)
    }
}

pub fn check_exist_pkg(db_path: &Path, package_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match check_package_local(db_path, package_name)? {
        Some(_) => Ok(true),  // Пакет найден
        None => Ok(false),    // Пакет не найден
    }
}

// Функция для выполнения скрипта установки
pub fn script_executor(path: &Path,script: &str) {

    let script_path = path.join("scripts").join(script);

    let src_path = path.join("src");
    let mask_path = path.join("mask");

    // Выполняем скрипт установки
    let _ = Command::new("bash")
        .arg(script_path)
        .arg(src_path)
        .arg(mask_path)
        .output()
        .unwrap();


}

pub fn get_package_dir(package_name: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    // Подключаемся к базе данных
    let conn = Connection::open(Path::new("/var/lib/konpac/packages.db"))?;
    
    // Выполняем запрос к базе данных
    let mut stmt = conn.prepare(
        "SELECT path FROM packages WHERE name = ?1"
    )?;

    // Ищем путь в базе данных
    let mut rows = stmt.query(params![package_name])?;
    
    if let Some(row) = rows.next()? {
        // Получаем путь из результата запроса
        let path_str: String = row.get(0)?;
        let path = PathBuf::from(path_str);
        
        // Проверяем существование пути
        if path.exists() {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}