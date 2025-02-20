#[cfg(windows)]
use std::env;
use std::error::Error;
use std::path::Path;
use std::process::{Command, ExitStatus};

type Output = Result<(), Box<dyn Error>>;

fn main() -> Output {
    #[cfg(target_os = "linux")]
    let libname = "ruby-static";
    #[cfg(target_os = "macos")]
    let libname = "ruby.3.2-static";
    #[cfg(all(target_arch = "x86_64", windows))]
    let libname = "x64-vcruntime140-ruby320-static";
    #[cfg(all(target_arch = "x86", windows))]
    let libname = "vcruntime140-ruby320-static";
    #[cfg(all(target_env = "gnu", windows))]
    compile_error!("rubyfmt on Windows is currently only supported with msvc");

    #[cfg(unix)]
    let ripper = "ext/ripper/ripper.o";
    #[cfg(windows)]
    let ripper = "ext/ripper/ripper.obj";

    let path = std::env::current_dir()?;
    let ruby_checkout_path = path.join("ruby_checkout");

    let old_checkout_sha = if ruby_checkout_path.join(ripper).exists() {
        Some(get_ruby_checkout_sha())
    } else {
        None
    };

    let _ = Command::new("git")
        .args(&["submodule", "update", "--init"])
        .status();

    let new_checkout_sha = get_ruby_checkout_sha();

    make_configure(&ruby_checkout_path)?;
    run_configure(&ruby_checkout_path)?;
    build_ruby(&ruby_checkout_path)?;
    // Only rerun this build if the ruby_checkout has changed
    // match old_checkout_sha {
        // Some(old_sha) if old_sha == new_checkout_sha => {}
        // _ => {
        // }
    // }

    cc::Build::new()
        .file("src/rubyfmt.c")
        .object(ruby_checkout_path.join(&ripper))
        .include(ruby_checkout_path.join("include"))
        .include(ruby_checkout_path.join(".ext/include/arm64-darwin20"))
        .include(ruby_checkout_path.join(".ext/include/arm64-darwin21"))
        .include(ruby_checkout_path.join(".ext/include/arm64-darwin22"))
        .include(ruby_checkout_path.join(".ext/include/aarch64-linux-gnu"))
        .include(ruby_checkout_path.join(".ext/include/x86_64-darwin21"))
        .include(ruby_checkout_path.join(".ext/include/x86_64-darwin20"))
        .include(ruby_checkout_path.join(".ext/include/x86_64-darwin19"))
        .include(ruby_checkout_path.join(".ext/include/x86_64-darwin18"))
        .include(ruby_checkout_path.join(".ext/include/x86_64-linux"))
        .include(ruby_checkout_path.join(".ext/include/x64-mswin64_140"))
        .include(ruby_checkout_path.join(".ext/include/i386-mswin32_140"))
        .warnings(false)
        .compile("rubyfmt_c");

    println!(
        "cargo:rustc-link-search=native={}",
        ruby_checkout_path.display()
    );
    println!("cargo:rustc-link-lib=static={}", libname);

    Ok(())
}

#[cfg(unix)]
fn make_configure(ruby_checkout_path: &Path) -> Output {
    let o = Command::new("autoreconf")
        .arg("--install")
        .current_dir(ruby_checkout_path)
        .status()?;
    check_process_success("autoreconf --install", o)
}


fn run_configure(ruby_checkout_path: &Path) -> Output {
    let o = Command::new("./configure")
        .arg("--without-gmp")
        .arg("--with-ext=ripper")
        .arg("--disable-jit-support")
        .arg("--target=aarch64-unknown-linux-gnu")
        .arg("--host=x86_64")
        .env("CC", "aarch64-linux-gnu-gcc")
        .env("AR", "aarch64-linux-gnu-ar")
        .env("RANLIB", "aarch64-linux-gnu-ranlib")
        .current_dir(ruby_checkout_path)
        .status()?;
    check_process_success("./configure", o)
}

#[cfg(unix)]
fn build_ruby(ruby_checkout_path: &Path) -> Output {
    let o = Command::new("make")
        .arg("clean")
        .current_dir(ruby_checkout_path)
        .status()?;
    check_process_success("make clean", o);

    let o = Command::new("make")
        .arg("main")
        .current_dir(ruby_checkout_path)
        .status()?;
    check_process_success("make main", o)
}

fn check_process_success(command: &str, code: ExitStatus) -> Output {
    if code.success() {
        Ok(())
    } else {
        Err(format!("Command {} failed with: {}", command, code).into())
    }
}

fn get_ruby_checkout_sha() -> String {
    String::from_utf8(
        Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir("./ruby_checkout")
            .output()
            .expect("git rev-parse shouldn't fail")
            .stdout,
    )
    .expect("output should be valid utf8")
}
