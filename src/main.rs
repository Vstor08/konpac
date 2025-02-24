mod package;
mod repo;
use package::install::{install_package_from_file, install_from_repo};
use package::remove::uninstall_package; // Предполагается, что эта функция существует в вашем модуле
use package::utils::is_elevated;
use repo::gen::generate_repo;
use clap::{Parser, ArgGroup};
use std::path::{Path, PathBuf};
use repo::utils::get_repos;
use tokio::time::{sleep, Duration};

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

    /// Сгенерировать репозиторий из папки с пакетами
    #[arg(long)]
    gen_repo: Option<String>,

    #[arg(long)]
    get_repo: Option<String>,

    #[arg(short, long)]
    download: Option<String>
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Welcome to konpac :)");

    match (args.install, args.remove, args.gen_repo, args.get_repo, args.download) {
        (Some(install_path), None, None, None, None) => {
            if !is_elevated() {
                eprintln!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            let install_package_path = Path::new(&install_path);
            install_package_from_file(install_package_path);
        },
        (None, Some(package_name), None, None, None) => {
            if !is_elevated() {
                eprintln!("Ошибка: Для удаления пакета требуются права администратора!");
                std::process::exit(1);
            }
            uninstall_package(package_name).unwrap_or_else(|e| {
                eprintln!("Ошибка удаления пакета: {}", e);
                std::process::exit(1);
            });
        },
        (None, None, Some(repo_path), None, None) => {
            let repo_path = Path::new(&repo_path).to_path_buf();
            generate_repo(repo_path);
        },
        (None, None, None, Some(get_repo), None) => {
            
            get_repos(Path::new(&get_repo));
        },
        (None, None, None, None, Some(package_name)) => {
            if !is_elevated() {
                eprintln!("Ошибка: Для установки пакета требуются права администратора!");
                std::process::exit(1);
            }
            match install_from_repo(package_name).await {
                Ok(_) => { println!("Installing succes") },
                Err(e) => { eprintln!("Error in package install: {}",e) }
            };
        },
        _ => unreachable!(),
    }
}