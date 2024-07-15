use std::path::PathBuf;
use std::process::Command;

fn main() {
    if !"../vendores/qbe/qbe".parse::<PathBuf>().unwrap().exists() {
        Command::new("make")
            .arg("qbe")
            .output()
            .expect("failed to execute process");
    }
}
