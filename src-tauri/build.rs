fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    #[cfg(target_os = "windows")]
    {
        if let Ok(env_path) = std::env::var("PATH") {
            env_path.split(";").for_each(|path| {
                println!("cargo:rustc-link-search=native={}", path);
            });
        }
    }
    tauri_build::build()
}
