use std::slice::from_raw_parts;
use std::mem::size_of;

// Permet de convertir une liste de nombres en liste d'octets
// Les octets sont souvent représentés par des u8 en Rust, 1 u8 = un octet
pub unsafe fn to_raw_data<T>(data: &[T]) -> &[u8] {
    from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>())
}
