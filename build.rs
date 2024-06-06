use std::process::Command;

fn main() {
    println!("cargo::rerun-if-changed=shaders/common.comp");
    println!("cargo::rerun-if-changed=shaders/wire.comp");
    println!("cargo::rerun-if-changed=shaders/component.comp");
    println!("cargo::rerun-if-changed=shaders/reset.comp");

    Command::new("glslangValidator")
        .current_dir("shaders/")
        .args([
            "-V",
            "-S",
            "comp",
            "-e",
            "main",
            "-o",
            "wire.spv",
            "wire.comp",
        ])
        .status()
        .unwrap();

    Command::new("glslangValidator")
        .current_dir("shaders/")
        .args([
            "-V",
            "-S",
            "comp",
            "-e",
            "main",
            "-o",
            "component.spv",
            "component.comp",
        ])
        .status()
        .unwrap();

    Command::new("glslangValidator")
        .current_dir("shaders/")
        .args([
            "-V",
            "-S",
            "comp",
            "-e",
            "main",
            "-o",
            "reset.spv",
            "reset.comp",
        ])
        .status()
        .unwrap();
}
