use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, BufRead};
use super::utils::{del_package,get_package_dir};

//TODO: Create remove functional

pub fn uninstall_package(package_name: String) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Получаем путь к директории пакета
    let package_dir = get_package_dir(package_name)?
        .ok_or_else(|| format!("Пакет '{}' не установлен", package_name))?;

    // 2. Удаление файлов по package.list
    let package_list = package_dir.join("package.list");
    
    if package_list.exists() {
        let file = fs::File::open(&package_list)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let path = Path::new(&line?.trim());
            if path.exists() {
                fs::remove_file(path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other, 
                        format!("Failed to delete {}: {}", path.display(), e)
                    )
                })?;
            }
        }
    }

    // 3. Удаление данных из БД
    del_package(package_name)?;

    // 4. Удаление директории пакета
    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other, 
                format!("Failed to remove package directory: {}", e)
            )
        })?;
    }

    Ok(())
}