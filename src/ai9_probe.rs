//! Isolated integration ABI. The caller owns the Count cache and supplies
//! source planner inputs. The active Game supplies the existing birth RNG.
use crate::ai9_count::{self, Cache, Precision53, Query, Sample, Word15};
use crate::{game::Game, native_rng9::NativeRng};
use core::ptr::addr_of_mut;

impl Word15 for NativeRng {
    fn next_word15(&mut self) -> u32 {
        NativeRng::next_word15(self)
    }
}

impl Game {
    fn source_ai_count(
        &mut self,
        cache: &mut Cache,
        query: Query,
        precision: Precision53,
    ) -> Result<Sample, ai9_count::Invalid> {
        ai9_count::sample(cache, query, precision, &mut self.source_birth.rng)
    }
}

// Word layout: supplied cache capacity and length, up to 512 cached words,
// then signed wanted count, sampled count, draw flag, and F64 bias bits.
// The 512-word limit belongs to this isolated transport.
const CAPACITY: usize = 512;
const OUTPUT: usize = 2 + CAPACITY;
const WORDS: usize = OUTPUT + 5;
static mut IO: [u32; WORDS] = [0; WORDS];

#[no_mangle]
pub extern "C" fn ai9_count_probe_ptr() -> *mut u32 {
    addr_of_mut!(IO).cast()
}

#[no_mangle]
pub extern "C" fn ai9_count_probe_words() -> u32 {
    WORDS as u32
}

#[no_mangle]
pub extern "C" fn ai9_count_probe_run(
    index: u32,
    min: i32,
    max: i32,
    multiplier: f32,
    now: f32,
    refresh_at: f32,
    factor: f32,
    has_factor: u32,
    precision: u32,
) -> u32 {
    let precision = match precision {
        0 => Precision53::NearestEven,
        1 => Precision53::TowardZero,
        _ => return 0,
    };
    if has_factor > 1 {
        return 0;
    }
    // SAFETY: this isolated WASM ABI runs on a single thread. The local
    // copy leaves caller memory unchanged if the query is rejected.
    let mut words = unsafe { IO };
    let (max_entries, length) = (words[0] as usize, words[1] as usize);
    if max_entries > CAPACITY || length > max_entries {
        return 0;
    }
    let mut cache = Cache {
        max_entries,
        counts: words[2..2 + length].to_vec(),
    };
    let query = Query {
        index: index as usize,
        min,
        max,
        multiplier,
        now,
        refresh_at,
        family_factor: (has_factor == 1).then_some(factor),
    };
    let Some(game) = (unsafe { (&mut *addr_of_mut!(crate::GAME)).as_deref_mut() }) else {
        return 0;
    };
    let Ok(result) = game.source_ai_count(&mut cache, query, precision) else {
        return 0;
    };
    words[1] = cache.counts.len() as u32;
    words[2..OUTPUT].fill(0);
    words[2..2 + cache.counts.len()].copy_from_slice(&cache.counts);
    words[OUTPUT..].copy_from_slice(&[
        result.projection.count as u32,
        result.cached_count,
        u32::from(result.sampled),
        result.projection.biased_bits as u32,
        (result.projection.biased_bits >> 32) as u32,
    ]);
    unsafe {
        IO = words;
    }
    1
}
