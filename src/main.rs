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
use log::{info, error};    // Логирование
use pretty_env_logger::formatted_builder; // Логгер
use std::sync::Once;
use std::io::Write;
use log::Level;

static INIT: Once = Once::new();

fn setup_logger() {
    INIT.call_once(|| {
        let mut builder = formatted_builder();
        builder.filter(None, log::LevelFilter::Info);
        builder.format(|buf, record| {
            let mut style = buf.style();
            match record.level() {
                Level::Error => style.set_color(pretty_env_logger::env_logger::fmt::Color::Red),
                Level::Warn => style.set_color(pretty_env_logger::env_logger::fmt::Color::Yellow),
                Level::Info => style.set_color(pretty_env_logger::env_logger::fmt::Color::Green),
                Level::Debug => style.set_color(pretty_env_logger::env_logger::fmt::Color::Blue),
                Level::Trace => style.set_color(pretty_env_logger::env_logger::fmt::Color::Magenta),
            };
            if cfg!(debug_assertions) {
                writeln!(buf, "{} {} > {}", style.value(record.level()), record.target(), record.args())
            } else {
                writeln!(buf, "{} > {}", style.value(record.level()), record.args())
            }
        });
        builder.init();
    });
}

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
    download: Option<String>,

    /// Установить пакет без подтверждения
    #[arg(long)]
    yes: bool,
}

// Основная асинхронная функция
#[tokio::main]
async fn main() {
    // Настройка логгера
    setup_logger();

    // Парсим аргументы командной строки
    let args = Args::parse();

    // Приветственное сообщение
    info!("Welcome to konpac :)");

    // Обрабатываем аргументы в зависимости от выбранной операции
    match (args.install, args.remove, args.gen_repo, args.get_repo, args.download) {
        // Установка пакета из файла
        (Some(install_path), None, None, None, None) => {
            // Проверяем, есть ли права администратора
            if !is_elevated() {
                error!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Устанавливаем пакет из указанного файла
            let install_package_path = Path::new(&install_path);
            match install_package_from_file(install_package_path, args.yes).await {
                Ok(_) => info!("Installed Success"),
                Err(e) => error!("Error in installation: {}", e)
            };
        },
        // Удаление пакета по имени
        (None, Some(package_name), None, None, None) => {
            // Проверяем, есть ли права администратора
            if !is_elevated() {
                error!("Ошибка: Для удаления пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Удаляем пакет и обрабатываем возможные ошибки
            uninstall_package(package_name).unwrap_or_else(|e| {
                error!("Ошибка удаления пакета: {}", e);
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
                error!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            // Устанавливаем пакет из репозитория и обрабатываем результат
            match install_from_repo(&package_name, args.yes).await {
                Ok(_) => { info!("Installing success") },
                Err(e) => { error!("Error in package install: {}", e) }
            };
        },
        // Обработка недопустимых комбинаций аргументов
        _ => unreachable!(),
    }
}