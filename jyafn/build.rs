use std::process::Command;

fn main() {
    Command::new("make")
        .arg("qbe")
        .current_dir("vendored/qbe")
        .output()
        .expect("failed to execute process");
}
