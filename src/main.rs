use rust_randomx::{Context, Difficulty, Hasher};
use std::sync::Arc;
use std::thread;

fn main() {
    let context = Arc::new(Context::new(b"RandomX key", true));

    let num_threads = thread::available_parallelism().expect("Failed to determine available parallelism").get();
    let mut threads = Vec::new();
    for i in 0..num_threads {
        let context = Arc::clone(&context);
        let diff = Difficulty::new(0x027fffff); // 0x00007fff ff000000 ... 00000000
        threads.push(thread::spawn(move || {
            let mut hasher = Hasher::new(&context);
            let mut nonce = i as u32;
            hasher.hash_first(&nonce.to_le_bytes());
            while nonce < u32::MAX {
                let next_nonce = nonce.saturating_add(num_threads as u32);
                let out = hasher.hash_next(&next_nonce.to_le_bytes());
                if out.meets_difficulty(diff) {
                    println!("{} -> {:?}", nonce, out);
                }
                nonce = next_nonce;
            }
        }));
    }

    for t in threads {
        t.join().unwrap();
    }
}
