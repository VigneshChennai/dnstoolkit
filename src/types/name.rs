use smallvec::SmallVec;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug)]
pub struct Label {
    value: SmallVec::<[u8; 14]>
}

#[derive(Debug)]
pub struct Name {
    // Size is set to  based on this research
    // https://www.farsightsecurity.com/blog/txt-record/rrlabel-20171013/
    //
    // ~88% chance the number of labels will be less than or equal to 4
    labels: SmallVec::<[Label; 4]>
}

#[derive(Debug, Error)]
pub enum NameParseError {
    #[error("Error converting string to ascii")]
    IDNAError(idna::Errors),
    #[error("Name '{0}' is larger than 255 characters")]
    NameTooLarge(String),
    #[error("Label '{0}' is larger than 63 characters")]
    LabelTooLong(String),
    #[error("EmptyLabel at position '{0}'")]
    EmptyLabel(usize)
}

impl FromStr for Name {
    type Err = NameParseError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let idna_domain = match idna::domain_to_ascii(name) {
            Ok(value) => value,
            Err(errors) => return Err(NameParseError::IDNAError(errors))
        };

        if idna_domain.len() > 255 {
            return Err(NameParseError::NameTooLarge(name.to_owned()))
        }

        let mut splits = idna_domain.split(".")
            .enumerate().peekable();

        let mut labels = SmallVec::<[Label; 4]>::new();

        while let Some((position, split)) = splits.next() {
            if split.len() > 63 {
                return Err(NameParseError::LabelTooLong(split.to_owned()))
            }

            if split.is_empty() && splits.peek().is_some() {
                return Err(NameParseError::EmptyLabel(position))
            }
            labels.push(Label {value: split.as_bytes().into()})
        };

        return Ok(Name { labels })
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
        assert!(Name::from_str(
            std::iter::repeat("x").take(63).collect::<String>().as_str()).is_ok());

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
        let long_label = std::iter::repeat("x").take(64).collect::<String>();
        assert!(match Name::from_str(format!("{}.com", long_label).as_str()) {
            Err(NameParseError::LabelTooLong(error_label)) => error_label == long_label,
            _ => false
        });

        let long_domain = std::iter::repeat("x.").take(128).collect::<String>();
        assert!(match Name::from_str(long_domain.as_str()) {
            Err(NameParseError::NameTooLarge(error_name)) => error_name == long_domain,
            _ => false
        });
    }

    #[test]
    fn not_allowed_characters() {
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
    fn allowed_characters() {
        assert!(Name::from_str("தமிழ்.wellsfargo.com").is_ok());
    }
}