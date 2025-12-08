use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Ensure memory.x is available to the linker
    println!("cargo:rustc-link-search={}", out.display());

    // Copy memory.x to the output directory
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();

    // Tell cargo to re-run this if memory.x changes
    println!("cargo:rerun-if-changed=memory.x");
}
