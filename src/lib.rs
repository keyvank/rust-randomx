#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_ulong, c_void};
use std::sync::Arc;
use std::thread;

#[allow(dead_code)]
mod bindings;
pub use bindings::*;

#[derive(Debug, Clone, Copy)]
pub struct Difficulty(u32);

fn div_128(a: u128, b: u128) -> f64 {
    let a_bytes = a.to_be_bytes();
    let b_bytes = b.to_be_bytes();
    let mut t_64 = 0u64;
    let mut r_64 = 0u64;
    let mut bytes = 0;
    let mut started = false;
    for (t, r) in a_bytes.into_iter().zip(b_bytes.into_iter()) {
        if t > 0 || r > 0 {
            started = true;
        }
        if started {
            t_64 <<= 8;
            r_64 <<= 8;
            t_64 += t as u64;
            r_64 += r as u64;
            bytes += 1;
            if bytes == 8 {
                break;
            }
        }
    }
    t_64 as f64 / r_64 as f64
}

impl Difficulty {
    pub fn to_u32(&self) -> u32 {
        self.0
    }
    pub fn new(d: u32) -> Self {
        Difficulty(d)
    }
    pub fn zeros(&self) -> usize {
        (self.0 >> 24) as usize
    }
    pub fn postfix(&self) -> u32 {
        self.0 & 0x00ffffff
    }
    pub fn powerf(&self) -> f64 {
        2f64.powf(self.zeros() as f64 * 8f64) * (0xffffff as f64 / self.postfix() as f64)
    }
    pub fn power(&self) -> u128 {
        self.powerf() as u128
    }
    pub fn from_power(target: u128) -> Self {
        let mut result = Self::new(0x00ffffff);
        loop {
            let mul = target / result.power();
            if mul > 2 {
                result = result.scale(2.0);
            } else {
                result = result.scale(div_128(target, result.power()) as f32);
                break;
            }
        }
        result
    }
    pub fn scale(&self, s: f32) -> Self {
        let mut zeros_add = s.log2() as i32 / 8;
        let rem = s / 256f32.powf(zeros_add as f32);
        let mut new_postfix = self.postfix() as f32 / rem;

        let postfix_power = 0xffffff as f32 / new_postfix;
        let postfix_power_zeros = postfix_power.log2() as i32 / 8;
        zeros_add += postfix_power_zeros;
        new_postfix *= 256f32.powf(postfix_power_zeros as f32);

        while new_postfix as u32 > 0xffffff {
            new_postfix /= 256f32;
            zeros_add -= 1;
        }

        if self.zeros() as i32 + zeros_add < 0 {
            return Self::new(0x00ffffff);
        }

        let new_postfix = (new_postfix as u32).to_le_bytes();

        Difficulty(u32::from_le_bytes([
            new_postfix[0],
            new_postfix[1],
            new_postfix[2],
            (self.zeros() as i32 + zeros_add) as u8,
        ]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Output([u8; RANDOMX_HASH_SIZE as usize]);

impl From<Difficulty> for Output {
    fn from(d: Difficulty) -> Self {
        let mut output = [0u8; 32];
        let zeros = d.zeros();
        let postfix = d.postfix();
        output[zeros..zeros + 3].copy_from_slice(&postfix.to_be_bytes()[1..4]);
        Self(output)
    }
}

impl AsRef<[u8]> for Output {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Output {
    pub fn meets_difficulty(&self, d: Difficulty) -> bool {
        for (a, b) in self.0.iter().zip(Output::from(d).0.iter()) {
            if a > b {
                return false;
            }
            if a < b {
                return true;
            }
        }
        true
    }

    pub fn leading_zeros(&self) -> u32 {
        let mut zeros = 0;
        for limb in self.0.iter() {
            let limb_zeros = limb.leading_zeros();
            zeros += limb_zeros;
            if limb_zeros != 8 {
                break;
            }
        }
        zeros
    }
}

#[derive(Clone)]
struct Sendable<T>(*mut T);
unsafe impl<T> Send for Sendable<T> {}

pub struct Context {
    key: Vec<u8>,
    flags: randomx_flags,
    fast: bool,
    cache: *mut randomx_cache,
    dataset: *mut randomx_dataset,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn key(&self) -> &[u8] {
        &self.key
    }
    pub fn new(key: &[u8], fast: bool) -> Self {
        unsafe {
            let mut flags = randomx_get_flags();
            let mut cache = randomx_alloc_cache(flags);
            randomx_init_cache(cache, key.as_ptr() as *const c_void, key.len());
            let mut dataset = std::ptr::null_mut();
            if fast {
                flags |= randomx_flags_RANDOMX_FLAG_FULL_MEM;
                dataset = randomx_alloc_dataset(flags);
                let num_threads = thread::available_parallelism().expect("Failed to determine available parallelism").get();
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
                            (i * length) as c_ulong,
                            length as c_ulong,
                        );
                    }));
                }
                for t in threads {
                    t.join()
                        .expect("Error while initializing the RandomX dataset!");
                }

                randomx_release_cache(cache);
                cache = std::ptr::null_mut();
            }

            Self {
                key: key.to_vec(),
                flags,
                fast,
                cache,
                dataset,
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            if self.fast {
                randomx_release_dataset(self.dataset);
            } else {
                randomx_release_cache(self.cache);
            }
        }
    }
}

pub struct Hasher {
    context: Arc<Context>,
    vm: *mut randomx_vm,
}

unsafe impl Send for Hasher {}
unsafe impl Sync for Hasher {}

impl Hasher {
    pub fn new(context: Arc<Context>) -> Self {
        unsafe {
            Hasher {
                context: Arc::clone(&context),
                vm: randomx_create_vm(context.flags, context.cache, context.dataset),
            }
        }
    }
    pub fn update(&mut self, context: Arc<Context>) {
        unsafe {
            if context.fast {
                randomx_vm_set_dataset(self.vm, context.dataset);
            } else {
                randomx_vm_set_cache(self.vm, context.cache);
            }
        }
        self.context = context;
    }
    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn hash(&self, inp: &[u8]) -> Output {
        let mut hash = [0u8; RANDOMX_HASH_SIZE as usize];
        unsafe {
            randomx_calculate_hash(
                self.vm,
                inp.as_ptr() as *const c_void,
                inp.len(),
                hash.as_mut_ptr() as *mut c_void,
            );
        }
        Output(hash)
    }

    pub fn hash_first(&mut self, inp: &[u8]) {
        unsafe {
            randomx_calculate_hash_first(self.vm, inp.as_ptr() as *const c_void, inp.len());
        }
    }
    pub fn hash_next(&mut self, next_inp: &[u8]) -> Output {
        let mut hash = [0u8; RANDOMX_HASH_SIZE as usize];
        unsafe {
            randomx_calculate_hash_next(
                self.vm,
                next_inp.as_ptr() as *const c_void,
                next_inp.len(),
                hash.as_mut_ptr() as *mut c_void,
            );
        }
        Output(hash)
    }
    pub fn hash_last(&mut self) -> Output {
        let mut hash = [0u8; RANDOMX_HASH_SIZE as usize];
        unsafe {
            randomx_calculate_hash_last(self.vm, hash.as_mut_ptr() as *mut c_void);
        }
        Output(hash)
    }
}

impl Drop for Hasher {
    fn drop(&mut self) {
        unsafe {
            randomx_destroy_vm(self.vm);
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
        let slow = Hasher::new(Arc::new(Context::new(KEY, false)));
        assert_eq!(slow.hash(INPUT), EXPECTED);
    }

    #[test]
    fn test_fast_hasher() {
        let fast = Hasher::new(Arc::new(Context::new(KEY, true)));
        assert_eq!(fast.hash(INPUT), EXPECTED);
    }

    #[test]
    fn test_difficulty_scaling() {
        let d1 = Difficulty::new(0x011fffff);
        let d2 = d1.scale(3f32).scale(3f32).scale(3f32);
        let d3 = d2.scale(1f32 / 3f32).scale(1f32 / 3f32).scale(1f32 / 3f32);
        assert_eq!(d1.power(), 2048);
        assert_eq!(d2.power(), 2048 * 27);
        assert_eq!(d3.power(), 2048);
    }
}
