use std::{env, path::PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=linker.ld");

    let target = env::var("TARGET").expect("Cargo must provide TARGET");

    if target != "aarch64-unknown-none" {
        return;
    }

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide CARGO_MANIFEST_DIR"),
    );
    let linker_script = manifest_dir.join("linker.ld");

    println!(
        "cargo::rustc-link-arg-bin=haechix-qemu-virt=-T{}",
        linker_script.display()
    );
}
