use rust_randomx::SlowHasher;

fn main() {
    let key = b"RandomX example key\x00";
    let input = b"RandomX example input\x00";
    let hasher = SlowHasher::new(key);
    println!("{:?}", hasher.hash(input));
}
