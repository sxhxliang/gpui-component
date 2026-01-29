fn main() {
    #[cfg(target_os = "windows")]
    {
        // Avoid duplicate MANIFEST resources when dependencies embed their own.
        println!("cargo:rustc-link-arg=/MANIFEST:NO");
    }
}
