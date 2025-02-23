extern crate libc;
extern crate rusqlite;


use rusqlite::{Connection, params, Result};
use std::path::{Path, PathBuf};
use std::error::Error;



#[derive(Debug)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
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

pub fn del_package(name: String) -> Result<()> {
    // TODO: create delete from database functional
    Ok(())
}

pub fn get_package_dir(package_name: String) -> Result<Option<PathBuf>, Box<dyn Error>> {
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