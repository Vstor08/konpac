mod package;
use package::install::{self, install_package_from_file};
use package::utils::{self, is_elevated};
use clap::Parser;

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
    install_package_from_file(&args.install,is_elevated());
}
