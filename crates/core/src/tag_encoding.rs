//! Canonical names shared by tag editing, validation, and history payloads.

use lofty::TextEncoding;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TagTextEncoding {
    Latin1,
    UTF16,
    UTF16BE,
    UTF8,
}

impl TagTextEncoding {
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "Latin1" => Some(Self::Latin1),
            "UTF16" => Some(Self::UTF16),
            "UTF16BE" => Some(Self::UTF16BE),
            "UTF8" => Some(Self::UTF8),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Latin1 => "Latin1",
            Self::UTF16 => "UTF16",
            Self::UTF16BE => "UTF16BE",
            Self::UTF8 => "UTF8",
        }
    }
}

// Lofty 0.25 has exactly these four values. Exhaustive conversions require an
// explicit decision if Lofty adds another value; unsupported names never default.
impl From<TextEncoding> for TagTextEncoding {
    fn from(encoding: TextEncoding) -> Self {
        match encoding {
            TextEncoding::Latin1 => Self::Latin1,
            TextEncoding::UTF16 => Self::UTF16,
            TextEncoding::UTF16BE => Self::UTF16BE,
            TextEncoding::UTF8 => Self::UTF8,
        }
    }
}

impl From<TagTextEncoding> for TextEncoding {
    fn from(encoding: TagTextEncoding) -> Self {
        match encoding {
            TagTextEncoding::Latin1 => Self::Latin1,
            TagTextEncoding::UTF16 => Self::UTF16,
            TagTextEncoding::UTF16BE => Self::UTF16BE,
            TagTextEncoding::UTF8 => Self::UTF8,
        }
    }
}
