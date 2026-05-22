use std::hash::Hash;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
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
        unsafe { str::from_utf8_unchecked(&self.data[..self.len]) }
    }
}

impl<const N: usize> AsRef<str> for ArchivedSmallString<N> {
    fn as_ref(&self) -> &str {
        let len = self.len.to_native() as usize;
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
