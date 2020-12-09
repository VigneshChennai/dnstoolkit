use std::convert::TryFrom;
use std::fmt::Display;
use std::str::FromStr;
use std::string::FromUtf8Error;

use smallvec::alloc::fmt::Formatter;
use smallvec::SmallVec;
use thiserror::Error;
use std::ops::Deref;

#[derive(Debug)]
pub struct Label<'a> {
    value: &'a [u8]
}

impl<'a> Display for Label<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = idna::domain_to_unicode(
            unsafe {
                // self.value is never set to non ASCII value.
                // So, the below step is safe.
                String::from_utf8_unchecked(Vec::from(self.value)).as_str()
            }).0;
        write!(f, "Label({})", v)
    }
}

#[derive(Debug)]
pub struct Name {
    value: SmallVec::<[u8; 36]>
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = idna::domain_to_unicode(
            unsafe {
                // self.value is never set to non ASCII value.
                // So, the below step is safe.
                String::from_utf8_unchecked(Vec::from(self.value.as_slice())).as_str()
            }
        ).0;
        write!(f, "{}", v)
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

impl Name {
    pub fn labels(&self) -> Vec<Label> {
        let splits = self.value.split(|v| *v == '.' as u8);
        splits.map(|v| Label { value: v }).collect()
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NameParseError {
    #[error("IDNAError: {0:?}")]
    IDNAError(idna::Errors),
    #[error("Utf8Error: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Name '{0}' is larger than 255 characters")]
    NameTooLarge(String),
    #[error("Label '{0}' is larger than 63 characters")]
    LabelTooLong(String),
    #[error("EmptyLabel at position '{0}'")]
    EmptyLabel(usize),
}

impl FromStr for Name {
    type Err = NameParseError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let idna_domain = match idna::domain_to_ascii(name) {
            Ok(value) => value,
            Err(errors) => return Err(NameParseError::IDNAError(errors))
        };

        if idna_domain.len() > 255 {
            return Err(NameParseError::NameTooLarge(name.to_owned()));
        }

        let mut splits = idna_domain.split(".")
            .enumerate().peekable();

        while let Some((position, split)) = splits.next() {
            if split.len() > 63 {
                return Err(NameParseError::LabelTooLong(split.to_owned()));
            }

            if split.is_empty() && splits.peek().is_some() {
                return Err(NameParseError::EmptyLabel(position));
            }
        };

        Ok(Name {
            value: SmallVec::<[u8; 36]>::from(idna_domain.as_bytes())
        })
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

#[cfg(test)]
mod tests {
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