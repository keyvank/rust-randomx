// build.rs

use cmake::Config;
use std::env;

fn main() {
    let dst = Config::new("RandomX").define("DARCH", "native").build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/build", dst.display());

    println!("cargo:rustc-link-lib=static=randomx");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    match target_os.as_str() {
        "openbsd" | "bitrig" | "netbsd" | "macos" | "ios" => {
            println!("cargo:rustc-link-lib=dylib=c++");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=advapi32");
            if target_env == "gnu" {
                println!("cargo:rustc-link-lib=dylib=stdc++");
            }
        }
        _ => {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
    }
}
