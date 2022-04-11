#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use num_cpus;
use std::os::raw::c_void;
use std::thread;

mod bindings;
use bindings::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Output([u8; RANDOMX_HASH_SIZE as usize]);

#[derive(Clone)]
struct Sendable<T>(*mut T);
unsafe impl<T> Send for Sendable<T> {}

pub struct Hasher {
    fast: bool,
    cache: *mut randomx_cache,
    vm: *mut randomx_vm,
    dataset: *mut randomx_dataset,
}
impl Hasher {
    pub fn new(key: &[u8], fast: bool) -> Self {
        unsafe {
            let mut flags = randomx_get_flags();
            let mut cache = randomx_alloc_cache(flags);
            randomx_init_cache(cache, key.as_ptr() as *const c_void, key.len() as u64);
            let mut dataset = std::ptr::null_mut();
            let vm = if fast {
                flags = flags | randomx_flags_RANDOMX_FLAG_FULL_MEM;
                dataset = randomx_alloc_dataset(flags);
                let num_threads = num_cpus::get();
                let length = randomx_dataset_item_count() as usize / num_threads;
                let mut threads = Vec::new();
                for i in 0..num_threads {
                    let sendable_cache = Sendable(cache);
                    let sendable_dataset = Sendable(dataset);
                    threads.push(thread::spawn(move || {
                        let cache = sendable_cache.clone();
                        let dataset = sendable_dataset.clone();
                        randomx_init_dataset(
                            dataset.0,
                            cache.0,
                            (i * length) as u64,
                            length as u64,
                        );
                    }));
                }
                for t in threads {
                    t.join()
                        .expect("Error while initializing the RandomX dataset!");
                }

                randomx_release_cache(cache);
                cache = std::ptr::null_mut();
                randomx_create_vm(flags, std::ptr::null_mut(), dataset)
            } else {
                randomx_create_vm(flags, cache, std::ptr::null_mut())
            };

            Hasher {
                fast,
                cache,
                vm,
                dataset,
            }
        }
    }

    pub fn hash(&self, inp: &[u8]) -> Output {
        let mut hash = [0u8; RANDOMX_HASH_SIZE as usize];
        unsafe {
            randomx_calculate_hash(
                self.vm,
                inp.as_ptr() as *const c_void,
                inp.len() as u64,
                hash.as_mut_ptr() as *mut c_void,
            );
        }
        Output(hash)
    }
}

impl Drop for Hasher {
    fn drop(&mut self) {
        unsafe {
            randomx_destroy_vm(self.vm);
            if self.fast {
                randomx_release_dataset(self.dataset);
            } else {
                randomx_release_cache(self.cache);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"RandomX example key\x00";
    const INPUT: &[u8] = b"RandomX example input\x00";
    const EXPECTED: Output = Output([
        138, 72, 229, 249, 219, 69, 171, 121, 217, 8, 5, 116, 196, 216, 25, 84, 254, 106, 198, 56,
        66, 33, 74, 255, 115, 194, 68, 178, 99, 48, 183, 201,
    ]);

    #[test]
    fn test_slow_hasher() {
        let slow = Hasher::new(KEY, false);
        assert_eq!(slow.hash(INPUT), EXPECTED);
    }

    #[test]
    fn test_fast_hasher() {
        let fast = Hasher::new(KEY, true);
        assert_eq!(fast.hash(INPUT), EXPECTED);
    }
}
