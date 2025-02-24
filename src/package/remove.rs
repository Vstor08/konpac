use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, BufRead};
use super::utils::{del_package,get_package_dir};



pub fn uninstall_package(package_name: String) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Получаем путь к директории пакета
    let package_dir = match get_package_dir(&package_name)? {
        Some(path) if path.exists() => path,
        Some(path) => return Err(format!("Директория пакета {:?} не существует", path).into()),
        None => return Err(format!("Пакет '{}' не найден в базе данных", package_name).into()),
    };

    // 2. Удаление файлов по package.list
    let package_list = package_dir.join("package.list");
    if package_list.exists() {
        let file = fs::File::open(&package_list)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let path = Path::new(line.trim());
            
            if path.exists() {
                fs::remove_file(path).map_err(|e| {
                    format!("Ошибка удаления {}: {}", path.display(), e)
                })?;
            }
        }
    }

    // 3. Удаление данных из БД
    del_package(package_name)?;

    // 4. Удаление директории пакета
    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|e| {
            format!("Ошибка удаления директории {}: {}", package_dir.display(), e)
        })?;
    }

    Ok(())
}