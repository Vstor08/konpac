use std::fs;
use std::path::Path;
use std::io::{self, BufRead};
use super::utils::{del_package, get_package_dir, script_executor};
use log::{info, error};

pub fn uninstall_package(package_name: String) -> Result<(), Box<dyn std::error::Error>> {
    info!("Начало удаления пакета: {}", package_name);

    // 1. Получаем путь к директории пакета
    let package_dir = match get_package_dir(&package_name)? {
        Some(path) if path.exists() => {
            info!("Найдена директория пакета: {:?}", path);
            path
        },
        Some(path) => {
            error!("Директория пакета {:?} не существует", path);
            return Err(format!("Директория пакета {:?} не существует", path).into());
        },
        None => {
            error!("Пакет '{}' не найден в базе данных", package_name);
            return Err(format!("Пакет '{}' не найден в базе данных", package_name).into());
        },
    };

    // 2. Удаление файлов по package.list
    let package_list = package_dir.join("package.list");
    if package_list.exists() {
        info!("Начало удаления файлов из списка: {:?}", package_list);
        let file = fs::File::open(&package_list)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let path = Path::new(line.trim());
            
            if path.exists() {
                info!("Удаление файла: {:?}", path);
                fs::remove_file(path).map_err(|e| {
                    error!("Ошибка удаления {}: {}", path.display(), e);
                    format!("Ошибка удаления {}: {}", path.display(), e)
                })?;
            } else {
                info!("Файл не найден: {:?}", path);
            }
        }
        info!("Удаление файлов из списка завершено");
    } else {
        info!("Файл package.list не найден: {:?}", package_list);
    }

    // 3. Удаление данных из БД
    info!("Удаление данных пакета из базы данных");
    del_package(package_name.clone())?;
    info!("Данные пакета удалены из базы данных");

    // 4. Удаление директории пакета
    if package_dir.exists() {
        info!("Выполнение скрипта удаления");
        script_executor(&package_dir, "remove");
        info!("Скрипт удаления выполнен");

        info!("Удаление директории пакета: {:?}", package_dir);
        fs::remove_dir_all(&package_dir).map_err(|e| {
            error!("Ошибка удаления директории {}: {}", package_dir.display(), e);
            format!("Ошибка удаления директории {}: {}", package_dir.display(), e)
        })?;
        info!("Директория пакета удалена: {:?}", package_dir);
    } else {
        info!("Директория пакета не найдена: {:?}", package_dir);
    }

    // 5. Выполнение скрипта удаления
    

    info!("Удаление пакета завершено: {}", package_name);
    Ok(())
}