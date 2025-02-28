mod package; // Подключаем модуль package
mod repo;    // Подключаем модуль repo
mod consts;
// Импортируем необходимые функции и структуры
use package::install::{install_package_from_file, install_from_repo};
use package::remove::uninstall_package; // Функция для удаления пакета
use package::utils::is_elevated;       // Функция для проверки прав администратора
use repo::gen::generate_repo;          // Функция для генерации репозитория
use clap::Parser;          // Библиотека для обработки аргументов командной строки
use std::path::Path;        // Работа с путями
use repo::utils::get_repos;            // Функция для получения репозиториев
    // Асинхронные задержки (не используется в текущем коде)
// Определяем структуру для обработки аргументов командной строки
#[derive(Parser)]
#[command(version)]
#[group(required = true, multiple = false)] // Группа аргументов, где можно выбрать только одну операцию
struct Args {
    /// Установить пакет из файла
    #[arg(short, long)]
    install: Option<String>,
    
    /// Удалить пакет по имени
    #[arg(short, long)]
    remove: Option<String>,

    /// Сгенерировать репозиторий из папки с пакетами
    #[arg(long)]
    gen_repo: Option<String>,

    /// Получить репозитории
    #[arg(long)]
    get_repo: Option<String>,

    /// Установить пакет из репозитория
    #[arg(short, long)]
    download: Option<String>
}

// Основная асинхронная функция
#[tokio::main]
async fn main() {
    // Парсим аргументы командной строки
    let args = Args::parse();

    // Приветственное сообщение
    println!("Welcome to konpac :)");

    // Обрабатываем аргументы в зависимости от выбранной операции
    match (args.install, args.remove, args.gen_repo, args.get_repo, args.download) {
        // Установка пакета из файла
        (Some(install_path), None, None, None, None) => {
            // Проверяем, есть ли права администратора
            if !is_elevated() {
                eprintln!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Устанавливаем пакет из указанного файла
            let install_package_path = Path::new(&install_path);
            install_package_from_file(install_package_path).await;
        },
        // Удаление пакета по имени
        (None, Some(package_name), None, None, None) => {
            // Проверяем, есть ли права администратора
            if !is_elevated() {
                eprintln!("Ошибка: Для удаления пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Удаляем пакет и обрабатываем возможные ошибки
            uninstall_package(package_name).unwrap_or_else(|e| {
                eprintln!("Ошибка удаления пакета: {}", e);
                std::process::exit(1);
            });
        },
        // Генерация репозитория из папки с пакетами
        (None, None, Some(repo_path), None, None) => {
            // Генерируем репозиторий из указанной папки
            let repo_path = Path::new(&repo_path).to_path_buf();
            generate_repo(repo_path);
        },
        // Получение репозиториев
        (None, None, None, Some(get_repo), None) => {
            // Получаем репозитории из указанного пути
            get_repos(Path::new(&get_repo));
        },
        // Установка пакета из репозитория
        (None, None, None, None, Some(package_name)) => {
            // Проверяем, есть ли права администратора
            if !is_elevated() {
                eprintln!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Устанавливаем пакет из репозитория и обрабатываем результат
            match install_from_repo(package_name).await {
                Ok(_) => { println!("Installing success") },
                Err(e) => { eprintln!("Error in package install: {}",e) }
            };
        },
        // Обработка недопустимых комбинаций аргументов
        _ => unreachable!(),
    }
}