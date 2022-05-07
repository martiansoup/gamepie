use std::error::Error;
use std::ffi::{CStr, OsStr, OsString};
use std::str::FromStr;

/// Errors on creating a PString
#[derive(Debug)]
pub enum PStringError {
    Nul,
    Utf8,
}

impl Error for PStringError {}

impl std::fmt::Display for PStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            PStringError::Nul => write!(f, "nul byte in string"),
            PStringError::Utf8 => write!(f, "non-utf8 string"),
        }
    }
}

pub struct PStr<'a> {
    // Have to represent as str as can't guarantee ending nul byte
    inner: &'a str,
}

impl<'a> PStr<'a> {
    /// Wrap a C string into a portable representation that can be guaranteed
    /// to be able to be treated as a C or Rust string.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the same reasons as `CStr::from_ptr()`.
    pub unsafe fn from_ptr(c: *const std::os::raw::c_char) -> Result<PStr<'a>, PStringError> {
        let cstr = CStr::from_ptr(c);
        match PString::check(cstr.to_bytes_with_nul()) {
            Ok(_) => {
                let s = cstr.to_str().unwrap();
                Ok(PStr { inner: s })
            }
            Err(e) => Err(e),
        }
    }

    pub fn split_once(&'a self, delimiter: &str) -> Option<(PStr, PStr)> {
        self.inner
            .split_once(delimiter)
            .map(|a| (PStr { inner: a.0 }, PStr { inner: a.1 }))
    }

    // Doesn't preserve safety as converts back to str
    pub fn split(&'a self, pattern: char) -> std::str::Split<char> {
        self.inner.split(pattern)
    }
}

impl<'a> TryFrom<&'a str> for PStr<'a> {
    type Error = PStringError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        PString::check(s.as_bytes())?;
        Ok(PStr { inner: s })
    }
}

impl<'a> TryFrom<&'a OsStr> for PStr<'a> {
    type Error = PStringError;

    fn try_from(s: &'a OsStr) -> Result<Self, Self::Error> {
        let st = s.to_str().ok_or(PStringError::Utf8)?;

        PString::check(st.as_bytes())?;
        Ok(PStr { inner: st })
    }
}

impl std::fmt::Display for PStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.inner)
    }
}

impl From<&PStr<'_>> for String {
    fn from(p: &PStr) -> Self {
        String::from(p.inner)
    }
}

impl From<PStr<'_>> for String {
    fn from(p: PStr) -> Self {
        String::from(p.inner)
    }
}

/// Portable string that can be safely represented in C and Rust
#[derive(Clone, PartialEq)]
pub struct PString {
    data: Vec<u8>,
}

impl PString {
    fn check(s: &[u8]) -> Result<(), PStringError> {
        assert_ne!(s.len(), 0, "can't have 0 length");
        assert_eq!(s.last(), Some(&0), "can't end in non-zero");

        // Ignore last zero for testing
        let string_slice = &s[0..s.len() - 1];
        if string_slice.iter().any(|c| *c == 0) {
            Err(PStringError::Nul)
        } else if std::str::from_utf8(string_slice).is_err() {
            Err(PStringError::Utf8)
        } else {
            Ok(())
        }
    }

    pub fn empty() -> PString {
        PString::from_str("").unwrap()
    }

    /// Wrap a C string into a portable representation that can be guaranteed
    /// to be able to be treated as a C or Rust string.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the same reasons as `CStr::from_ptr()`.
    pub unsafe fn from_ptr(c: *const std::os::raw::c_char) -> Result<PString, PStringError> {
        let cstr = CStr::from_ptr(c);
        cstr.try_into()
    }

    pub fn to_str(&self) -> &str {
        // Checked on creation of string
        unsafe { std::str::from_utf8_unchecked(&self.data[0..self.data.len() - 1]) }
    }

    pub fn as_ptr(&self) -> *const std::os::raw::c_char {
        // Checked on creation of string
        unsafe { CStr::from_bytes_with_nul_unchecked(&self.data).as_ptr() }
    }
}

impl From<PString> for String {
    fn from(s: PString) -> Self {
        // Checked on creation of string
        unsafe { String::from_utf8_unchecked((&s.data[0..s.data.len() - 1]).to_vec()) }
    }
}

impl<'a> From<&PStr<'a>> for PString {
    fn from(s: &PStr<'a>) -> Self {
        let mut data = s.inner.as_bytes().to_vec();
        data.push(0);
        PString { data }
    }
}

impl TryFrom<Vec<u8>> for PString {
    type Error = PStringError;

    fn try_from(s: Vec<u8>) -> Result<Self, Self::Error> {
        Self::check(&s).map(|_| PString { data: s })
    }
}

impl TryFrom<&CStr> for PString {
    type Error = PStringError;

    fn try_from(s: &CStr) -> Result<Self, Self::Error> {
        let vec = s.to_bytes_with_nul().to_vec();

        vec.try_into()
    }
}

impl TryFrom<OsString> for PString {
    type Error = PStringError;

    fn try_from(s: OsString) -> Result<Self, Self::Error> {
        if let Some(s) = s.to_str() {
            let mut data = s.as_bytes().to_vec();
            data.push(0);
            data.try_into()
        } else {
            Err(PStringError::Utf8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PString, PStringError};
    use std::ffi::CStr;
    use std::str::FromStr;

    #[test]
    fn valid_string() {
        let s = PString::from_str("valid string");
        assert!(s.is_ok());
    }

    #[test]
    fn invalid_string_nul() {
        let s = PString::from_str("invalid\0 string");
        assert!(s.is_err());
    }

    #[test]
    fn invalid_string_nul_end() {
        let s = PString::from_str("invalid string\0");
        assert!(s.is_err());
    }

    #[test]
    fn valid_cstring() {
        let bytes = vec![0x48, 0x69, 0x00];
        let c = CStr::from_bytes_with_nul(&bytes).unwrap();

        let s: Result<PString, PStringError> = c.try_into();
        assert!(s.is_ok());
    }

    #[test]
    fn invalid_string_non_utf8() {
        let bytes = vec![0xc3, 0x28, 0x00];
        let c = CStr::from_bytes_with_nul(&bytes).unwrap();

        let s: Result<PString, PStringError> = c.try_into();
        assert!(s.is_err());
    }

    #[test]
    fn to_string_ok_rust_rust() {
        let original = "this is a test";
        let p = PString::from_str(original).unwrap();
        assert_eq!(p.to_str(), original);
    }

    #[test]
    fn to_string_ok_c_rust() {
        let bytes = vec![0x48, 0x69, 0x00];
        let c = CStr::from_bytes_with_nul(&bytes).unwrap();

        let s: PString = c.try_into().unwrap();

        assert_eq!(s.to_str(), "Hi");
    }
}

impl FromStr for PString {
    type Err = PStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut vec = s.as_bytes().to_vec();
        vec.push(0);

        vec.try_into()
    }
}
