// build.rs

use cmake::Config;

fn main() {
    let dst = Config::new("RandomX").define("DARCH", "native").build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=randomx");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
