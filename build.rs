use std::env;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=config.yml");

    let output_path = get_output_path();
    println!(
        "cargo:trace=Calculated build path: {}",
        output_path.to_str().unwrap()
    );

    let input_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("config.yml");
    let output_path = Path::new(&output_path).join("config.yml");
    if let Err(_) = std::fs::copy(input_path, output_path) {
        eprintln!("Failed to copy config.yml");
    }
}

fn get_output_path() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let path = Path::new(&manifest_dir_string)
        .join("target")
        .join(build_type);
    return PathBuf::from(path);
}
