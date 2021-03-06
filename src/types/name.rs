use std::convert::TryFrom;
use std::fmt::Display;
use std::str::FromStr;
use std::string::FromUtf8Error;

use smallvec::alloc::fmt::Formatter;
use smallvec::SmallVec;
use thiserror::Error;
use std::ops::Deref;
use idna::Config;
use std::any::Any;


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label<'a> {
    value: &'a [u8]
}

impl<'a> Display for Label<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", unsafe {std::str::from_utf8_unchecked(self.value)})
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name {
    value: SmallVec<[u8; 36]>
}

/// The errors that can happen when parsing
///
/// 1. &str to Name
/// 2. &[u8] to Name
///
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NameParseError {
    #[error("IDNAError")]
    IDNAError(#[from] idna::Errors),
    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Name '{0}' is larger than 255 characters")]
    NameTooLarge(String),
    #[error("Label '{0}' is larger than 63 characters")]
    LabelTooLong(String),
    #[error("EmptyLabel at position '{0}'")]
    EmptyLabel(usize)
}

impl Name {
    pub fn labels(&self) -> Vec<Label> {
        let splits = self.value.split(|v| *v == '.' as u8);
        splits.map(|v| Label { value: v }).collect()
    }

    pub fn is_absolute(&self) -> bool {
        self.value.len() > 0 && self.value[self.len() - 1] == '.' as u8
    }

    /// This function is unsafe because there is no checks made to ensure the given
    /// &[u8] is ascii u8 slice
    ///
    /// This the responsibility of the caller to ensure the &[u8] is a u8 slice of ascii
    /// characters
    pub unsafe fn from_bytes_ascii(name: &[u8]) -> Result<Self, NameParseError> {

        if name.len() > 255 {
            return Err(NameParseError::NameTooLarge(
                String::from_utf8_unchecked(name.into())));
        }

        let mut splits
            = name.split(|v| b'.' == *v).enumerate().peekable();

        while let Some((position, split)) = splits.next() {
            if split.len() > 63 {
                return Err(NameParseError::LabelTooLong(
                    String::from_utf8_unchecked(split.into())));
            }

            if split.is_empty() && splits.peek().is_some() {
                return Err(NameParseError::EmptyLabel(position));
            }
        };

        Self::from_bytes_raw(name)
    }

    /// This function is unsafe because, there is no checks made to ensure the given &[u8]
    /// is
    ///
    /// 1. ascii u8 slice or empty u8 slice
    /// 2. Length is <=255
    /// 3. Label length is <=63
    /// 4. Not empty label(other than root)
    ///
    /// All the above responsibility is the responsibility of the caller to ensure
    /// not undefined behaviour occurs.
    #[inline]
    pub unsafe fn from_bytes_raw(name: &[u8]) -> Result<Self, NameParseError> {
        Ok(Name {
            value: SmallVec::<[u8; 36]>::from(name)
        })
    }

    #[inline]
    pub fn from_bytes(name: &[u8]) -> Result<Self, NameParseError> {
        let string = String::from_utf8(Vec::from(name))?;
        string.parse()
    }

    /// This function is unsafe because, there is no checks made to ensure the given &str
    /// is ascii.
    ///
    /// This the responsibility of the caller to ensure the &str is a ascii text
    /// to prevent any undefined behaviour
    #[inline]
    pub unsafe fn from_text_ascii(name: &str) -> Result<Self, NameParseError> {
        Self::from_bytes_ascii(name.as_bytes())
    }

    #[inline]
    pub fn from_text(name: &str) -> Result<Self, NameParseError> {
        let idna = idna::Config::default();
        // Disabling hyphen '-' check on label
        // If set to true, labels starts with and ends with hyphens are marked as errors
        idna.check_hyphens(false);

        // Disabling transitional processing.
        //
        // What it mean is that, the codepoints/characters which are valid in idna2003
        // but has a different codepoint/character in idna2008 be
        // will changed/replaced as per idna2008
        // mapping.
        //
        // if set to true, those codepoints/characters won't be modified.
        idna.transitional_processing(false);

        // Disabled std3 specific rules
        //
        // This means that the codepoints/characters which are invalid in idna2003 but are valid or
        // mapped to other codepoint/characters in idna2008 will be unmodified or
        // changed/replaced as per idna2008 mapping.
        //
        // if set to true, labels contain those codepoints/characters will be marked as errors.
        idna.use_std3_ascii_rules(false);

        // Disabled Label max length and Domain name max length and other similar check.
        //
        // Disabled at idna level as we are performing these checks in this crates code
        idna.verify_dns_length(false);

        // Converting unicode string to idna compatible format.
        // Any error occurred will be propagated.
        let idna_domain = idna::domain_to_ascii(name)?;

        // This is safe because, idna::domain_to_ascii function will return
        // String only with ascii characters
        return unsafe { Self::from_text_ascii(idna_domain.as_str()) }
    }

    // TODO: implement ```fn from_wire(&self, ...)```
    // TODO: implement ```fn is_wild(&self)```
    // TODO: implement ```fn fullcompare(&self, other: Self)```
    // TODO: implement ```fn is_subdomain(&self)```
    // TODO: implement ```fn is_superdomain(&self)```
    // TODO: implement ```fn to_text(&self)```
    // TODO: implement ```fn to_unicode(&self)```
    // TODO: implement ```fn to_wire(&self, ...)```
    // TODO: implement ```fn to_digestable(&self, origin: Self)```
    // TODO: implement ```fn split(&self, depth: usize)```
    // TODO: implement ```fn concatenate(&self, other: Self)```
    // TODO: implement ```fn relativize(&self, origin: Self)```
    // TODO: implement ```fn derelativize(&self, origin: Self)```
    // TODO: implement ```fn choose_relativity(&self, ...)```
    // TODO: implement ```fn parent(&self)```
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", unsafe {std::str::from_utf8_unchecked(self.value.as_ref())})
    }
}

impl FromStr for Name {
    type Err = NameParseError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::from_text(name)
    }
}

impl TryFrom<&str> for Name {
    type Error = NameParseError;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&[u8]> for Name {
    type Error = NameParseError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let string = String::from_utf8(Vec::from(value))?;
        string.parse()
    }
}

impl TryFrom<String> for Name {
    type Error = NameParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Deref for Name {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

impl AsRef<[u8]> for Name {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        // This is safe as we never allow non ascii characters in self.value
        unsafe {std::str::from_utf8_unchecked(self.value.as_ref())}
    }
}

lazy_static! {
    /// Both the below two are safe as all the conditions of Name::from_bytes_raw
    /// for safe usage are met.
    ///
    /// Both the b"." and b""
    ///
    /// 1. ascii u8 slice or empty u8 slice
    /// 2. Length is <=255
    /// 3. Label length is <=63
    /// 4. Not empty label(other than root)
    pub static ref ROOT: Name = unsafe { Name::from_bytes_raw(b".").unwrap() };
    pub static ref EMPTY: Name = unsafe { Name::from_bytes_raw(b"").unwrap() };
}

#[cfg(test)]
mod tests_parsing {
    use super::*;

    #[test]
    fn valid_cases_relative() {
        assert!(Name::from_str("google.com").is_ok());
        assert!(Name::from_str("www.google.com").is_ok());
        assert!(Name::from_str("internal.www.facebook.com").is_ok());
        assert!(Name::from_str("blog.bbc.co.uk").is_ok());
        assert!(Name::from_str("annauniversity.au").is_ok());
    }

    #[test]
    fn valid_cases_absolute() {
        assert!(Name::from_str("google.com.").is_ok());
        assert!(Name::from_str("www.google.com.").is_ok());
        assert!(Name::from_str("internal.www.facebook.com.").is_ok());
        assert!(Name::from_str("blog.bbc.co.uk.").is_ok());
        assert!(Name::from_str("annauniversity.au.").is_ok());
    }

    #[test]
    fn valid_cases_max_limit() {
        // Max label size allowed
        assert!(Name::from_str(
            std::iter::repeat("x").take(63).collect::<String>().as_str()).is_ok());

        // Max domain size allowed
        assert!(Name::from_str(
            std::iter::repeat("x.").take(127).collect::<String>().as_str()).is_ok());
    }

    #[test]
    fn invalid_cases_emptylabels() {
        assert!(match Name::from_str("..google.com") {
            Err(NameParseError::EmptyLabel(pos)) => pos == 0,
            _ => false
        });

        assert!(match Name::from_str(".google.com") {
            Err(NameParseError::EmptyLabel(pos)) => pos == 0,
            _ => false
        });

        assert!(match Name::from_str("www..google.com") {
            Err(NameParseError::EmptyLabel(pos)) => pos == 1,
            _ => false
        });
    }

    #[test]
    fn invalid_cases_near_max_limit() {
        // just above 63 character max limit for a label
        let long_label = std::iter::repeat("x").take(64).collect::<String>();
        assert!(match Name::from_str(format!("{}.com", long_label).as_str()) {
            Err(NameParseError::LabelTooLong(error_label)) => error_label == long_label,
            _ => false
        });

        // just above 255 character max limit for domain name
        let long_domain = std::iter::repeat("x.").take(128).collect::<String>();
        assert!(match Name::from_str(long_domain.as_str()) {
            Err(NameParseError::NameTooLarge(error_name)) => error_name == long_domain,
            _ => false
        });
    }

    #[test]
    fn not_allowed_unicode_characters() {
        assert!(match Name::from_str("secure\u{2488}wellsfargo.com") {
            Err(NameParseError::IDNAError(_errors)) => true,
            Err(e) => {
                println!("{:?}", e);
                false
            }
            Ok(v) => {
                println!("{:?}", v);
                false
            }
        });
    }

    #[test]
    fn allowed_unicode_characters() {
        assert!(Name::from_str("தமிழ்.wellsfargo.com").is_ok());
    }
}

#[cfg(test)]
mod tests_layout {
    use super::*;
}