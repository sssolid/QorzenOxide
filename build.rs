use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let exe_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3) // Traverse from OUT_DIR to target/debug/
        .expect("Failed to determine target directory");

    let source = Path::new("public/static");
    let destination = exe_dir.join("static");

    if destination.exists() {
        fs::remove_dir_all(&destination).expect("Failed to clean old static folder");
    }

    copy_dir_recursive(source, &destination).expect("Failed to copy static assets");

    println!("cargo:rerun-if-changed=public/static");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
