use rust_randomx::Hasher;

fn main() {
    let key = b"RandomX example key\x00";
    let input = b"RandomX example input\x00";
    let hasher = Hasher::new(key, false);
    println!("{:?}", hasher.hash(input));
}
