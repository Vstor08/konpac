mod package;
use package::install::{self, install_package_from_file};

fn main() {
    println!("Hello, world!");
    install_package_from_file("./example/package/package.yml");
}
