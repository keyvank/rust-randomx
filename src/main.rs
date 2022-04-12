use num_cpus;
use rand::prelude::*;
use rust_randomx::{Context, Difficulty, Hasher};
use std::sync::Arc;
use std::thread;

fn main() {
    let context = Arc::new(Context::new(b"RandomX key", true));

    let num_threads = num_cpus::get();
    let mut threads = Vec::new();
    for _ in 0..num_threads {
        let context = Arc::clone(&context);
        let diff = Difficulty::new(0x027fffff); // 0x00007fff ff000000 ... 00000000
        threads.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let hasher = Hasher::new(context);
            loop {
                let nonce: u32 = rng.gen();
                let out = hasher.hash(&nonce.to_le_bytes());
                if out.meets_difficulty(diff) {
                    println!("{} -> {:?}", nonce, out);
                }
            }
        }));
    }

    for t in threads {
        t.join().unwrap();
    }
}
