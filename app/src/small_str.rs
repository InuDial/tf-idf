use std::hash::Hash;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Archive, Serialize, Deserialize)]
pub struct SmallString<const N: usize> {
    len: usize,
    /// always legal str
    data: [u8; N],
}

impl<const N: usize> TryFrom<&str> for SmallString<N> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        let len = bytes.len();
        if len > N {
            return Err(());
        }
        let mut data = [0; N];
        data[..len].copy_from_slice(bytes);
        Ok(Self { len, data })
    }
}

impl<const N: usize> TryFrom<&str> for ArchivedSmallString<N> {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        let len = bytes.len();
        if len > N {
            return Err(());
        }
        let mut data = [0; N];
        data[..len].copy_from_slice(bytes);
        Ok(Self {
            len: (len as u32).into(),
            data,
        })
    }
}

#[derive(Debug)]
pub enum FromBytesError {
    TooLarge,
    IllegalString,
}

impl<const N: usize> TryFrom<&[u8]> for SmallString<N> {
    type Error = FromBytesError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let len = bytes.len();
        if len > N {
            Err(FromBytesError::TooLarge)
        } else {
            let s = str::from_utf8(bytes).map_err(|_| FromBytesError::IllegalString)?;

            s.try_into().map_err(|_| FromBytesError::TooLarge)
        }
    }
}

impl<const N: usize> AsRef<str> for SmallString<N> {
    fn as_ref(&self) -> &str {
        // SAFETY: guaranteed to be valid in SmallString
        unsafe { str::from_utf8_unchecked(&self.data[..self.len]) }
    }
}

impl<const N: usize> AsRef<str> for ArchivedSmallString<N> {
    fn as_ref(&self) -> &str {
        let len = self.len.to_native() as usize;
        // TODO: SAFETY
        unsafe { str::from_utf8_unchecked(&self.data[..len]) }
    }
}

impl<const N: usize> From<SmallString<N>> for String {
    fn from(value: SmallString<N>) -> Self {
        let s: &str = value.as_ref();
        String::from(s)
    }
}

impl<const N: usize> Hash for SmallString<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let s: &str = self.as_ref();
        s.hash(state);
    }
}

impl<const N: usize> Hash for ArchivedSmallString<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let s: &str = self.as_ref();
        s.hash(state);
    }
}

impl<const N: usize> PartialEq for SmallString<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<const N: usize> Eq for SmallString<N> {}

impl<const N: usize> PartialEq for ArchivedSmallString<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<const N: usize> Eq for ArchivedSmallString<N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_str_normal() {
        let s: SmallString<10> = "hello".try_into().unwrap();
        assert_eq!(s.as_ref(), "hello");
    }

    #[test]
    fn try_from_str_empty() {
        let s: SmallString<10> = "".try_into().unwrap();
        assert_eq!(s.as_ref(), "");
    }

    #[test]
    fn try_from_str_exact_max() {
        let s: SmallString<5> = "abcde".try_into().unwrap();
        assert_eq!(s.as_ref(), "abcde");
    }

    #[test]
    fn try_from_str_too_long() {
        let r: Result<SmallString<3>, _> = "abcd".try_into();
        assert!(r.is_err());
    }

    #[test]
    fn eq_same() {
        let a: SmallString<10> = "rust".try_into().unwrap();
        let b: SmallString<10> = "rust".try_into().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn eq_different() {
        let a: SmallString<10> = "rust".try_into().unwrap();
        let b: SmallString<10> = "go".try_into().unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hash_consistent() {
        use std::hash::{DefaultHasher, Hasher};
        let a: SmallString<10> = "test".try_into().unwrap();
        let b: SmallString<10> = "test".try_into().unwrap();
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn into_string() {
        let s: SmallString<10> = "hello".try_into().unwrap();
        let string: String = s.into();
        assert_eq!(string, "hello");
    }

    #[test]
    fn from_bytes_valid() {
        let bytes = b"test";
        let s: SmallString<10> = SmallString::try_from(&bytes[..]).unwrap();
        assert_eq!(s.as_ref(), "test");
    }

    #[test]
    fn from_bytes_invalid_utf8() {
        let bytes = [0xFF, 0xFE];
        let r: Result<SmallString<10>, _> = SmallString::try_from(&bytes[..]);
        assert!(matches!(r, Err(FromBytesError::IllegalString)));
    }

    #[test]
    fn from_bytes_too_large() {
        let bytes = [0u8; 11];
        let r: Result<SmallString<10>, _> = SmallString::try_from(&bytes[..]);
        assert!(matches!(r, Err(FromBytesError::TooLarge)));
    }
}
