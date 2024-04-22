use std::slice::from_raw_parts;
use std::mem::size_of;

pub unsafe fn to_raw_data<T>(data: &[T]) -> &[u8] {
    from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>())
}
