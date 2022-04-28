// build.rs

use std::env;
use cmake::Config;

fn main() {
    let dst = Config::new("RandomX").define("DARCH", "native").build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=randomx");

    let target_os = env::var("CARGO_CFG_TARGET_OS");
    match target_os.as_ref().map(|x| &**x) {
        Ok("linux") | Ok("android") => {
            println!("cargo:rustc-link-lib=dylib=libc++")
        }
        Ok("openbsd") | Ok("bitrig") | Ok("netbsd") | Ok("macos") | Ok("ios") => {
            println!("cargo:rustc-link-lib=dylib=c++");
        }
        tos => panic!("unsupported target os {:?}!", tos)
    }
}
