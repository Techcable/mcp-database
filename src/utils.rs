use std::{fmt, slice, mem};
use std::marker::PhantomData;
use serde::de::{self, Deserializer, Deserialize, Visitor};

pub unsafe trait TransmuteFixedBytes {}

#[inline]
pub fn deserialize_borrowed_list<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: TransmuteFixedBytes, D: Deserializer<'de> {
    struct FixedBytesList<T>(PhantomData<T>);
    impl<'de, T: TransmuteFixedBytes> Visitor<'de> for U32Visitor {
        type Value = &'de [T];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an VersionIdList")
        }

        fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where E: de::Error, {
            if v.len() % mem::size_of::<T>() == 0 {
                unsafe {
                    Ok(slice::from_raw_parts(
                        v.as_ptr() as *const T,
                        v.len() / mem::size_of::<T>()
                    ))
                }
            } else {
                Err(E::invalid_length(v.len(), "a multiple of mem::size_of::<T>()"))
            }
        }
    }
    deserializer.deserialize_bytes(FixedBytesList)
}

#[inline]
pub fn binary_search_left_by_key<T, B, F>(target: &[T], b: &B, func: F) -> Option<(usize, &T)>
    where F: FnMut(&T) -> B {
    match target.binary_search_by_key(b, func) {
        Ok(index) => Some((index, unsafe { target.get_unchecked(index) })),
        Err(index) => target.get(index).map(|value| (index, value)),
    }
}