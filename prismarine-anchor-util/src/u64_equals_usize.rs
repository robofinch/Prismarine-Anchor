/// In practice, this is useful for comparing the position of a `Cursor` with the length of
/// data you are trying to read.
#[inline]
pub fn u64_equals_usize(num_u64: u64, num_usize: usize) -> bool {
    // The as casts don't overflow because we check the size.
    if size_of::<usize>() <= size_of::<u64>() {
        let num_usize = num_usize as u64;
        num_u64 == num_usize

    } else {
        let num_u64 = num_u64 as usize;
        num_u64 == num_usize
    }
}
