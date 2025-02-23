mod package;
use package::install::install_package_from_file;
use package::utils::is_elevated;
use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Входной файл
    #[arg(short, long)]
    install: String,
}
fn main() {
    let args = Args::parse();

    println!("Welcome to konpac :)");
    let install_package_path = Path::new(&args.install);
    install_package_from_file(install_package_path,is_elevated());
}
