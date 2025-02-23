mod package;
use package::install::install_package_from_file;
use package::remove::uninstall_package; // Предполагается, что эта функция существует в вашем модуле
use package::utils::is_elevated;
use clap::{Parser, ArgGroup};
use std::path::Path;

#[derive(Parser)]
#[command(version)]
#[group(required = true, multiple = false)] // Обеспечиваем выбор только одной операции
struct Args {
    /// Установить пакет из файла
    #[arg(short, long)]
    install: Option<String>,
    
    /// Удалить пакет по имени
    #[arg(short, long)]
    remove: Option<String>,
}

fn main() {
    let args = Args::parse();

    println!("Welcome to konpac :)");
    
    if !is_elevated() {
        eprintln!("Ошибка: Для этой операции требуются права администратора!");
        std::process::exit(1);
    }

    match (args.install, args.remove) {
        (Some(install_path), None) => {
            let install_package_path = Path::new(&install_path);
            install_package_from_file(install_package_path);
        },
        (None, Some(package_name)) => {
            uninstall_package(package_name).unwrap_or_else(|e| {
                eprintln!("Ошибка удаления пакета: {}", e);
                std::process::exit(1);
            });
        },
        _ => unreachable!(), // Благодаря группе ArgGroup эта ветка никогда не выполнится
    }
}