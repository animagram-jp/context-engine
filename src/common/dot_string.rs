use std::ops::{Index, Range, RangeFrom, RangeTo, RangeFull};

/// # Examples
///
/// ```
/// use state_engine::common::DotString;
///
/// let dot_string = DotString::new("segment1.segment2.segment3.segment4.segment5");
/// assert_eq!(&dot_string[0], "segment1");
/// assert_eq!(&dot_string[1], "segment2");
/// assert_eq!(&dot_string[2], "segment3");
/// assert_eq!(dot_string.len(), 5);
/// assert_eq!(&dot_string[3..5], &["segment4".to_string(), "segment5".to_string()]);
/// ```
pub struct DotString {
    key: String,
    segments: Vec<String>,
}

impl DotString {
    pub fn new(dot_key: &str) -> Self {
        let segments = if dot_key.is_empty() {
            Vec::new()
        } else {
            dot_key.split('.').map(|s| s.to_string()).collect()
        };

        Self {
            key: dot_key.to_string(),
            segments,
        }
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn as_str(&self) -> &str {
        &self.key
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.segments.iter().map(|s| s.as_str())
    }
}

// dot_string[0]
impl Index<usize> for DotString {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        &self.segments[index]
    }
}

// dot_string[0..2]
impl Index<Range<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// dot_string[0..]
impl Index<RangeFrom<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: RangeFrom<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// dot_string[..2]
impl Index<RangeTo<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: RangeTo<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// dot_string[..]
impl Index<RangeFull> for DotString {
    type Output = [String];

    fn index(&self, _range: RangeFull) -> &Self::Output {
        &self.segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let dot_string = DotString::new("");
        assert_eq!(dot_string.len(), 0);
    }

    #[test]
    fn test_single_segment() {
        let dot_string = DotString::new("segment1");
        assert_eq!(dot_string.len(), 1);
        assert_eq!(&dot_string[0], "segment1");
    }

    #[test]
    fn test_index_access() {
        let dot_string = DotString::new("segment1.segment2.segment3");
        assert_eq!(&dot_string[0], "segment1");
        assert_eq!(&dot_string[1], "segment2");
        assert_eq!(&dot_string[2], "segment3");
    }

    #[test]
    fn test_range_access() {
        let dot_string = DotString::new("1.2.3");

        assert_eq!(&dot_string[0..2], &["1".to_string(), "2".to_string()]);
        assert_eq!(&dot_string[1..], &["2".to_string(), "3".to_string()]);
        assert_eq!(&dot_string[..2], &["1".to_string(), "2".to_string()]);
        assert_eq!(&dot_string[..], &["1".to_string(), "2".to_string(), "3".to_string()]);
    }

    #[test]
    fn test_negative_index_emulation() {
        let dot_string = DotString::new("segment1.segment2.segment3");

        let last = &dot_string[dot_string.len() - 1];
        assert_eq!(last, "segment3");

        let without_last = &dot_string[..dot_string.len() - 1];
        assert_eq!(without_last, &["segment1".to_string(), "segment2".to_string()]);
    }

    #[test]
    fn test_iter() {
        let dot_string = DotString::new("segment1.segment2.segment3");
        let collected: Vec<&str> = dot_string.iter().collect();
        assert_eq!(collected, vec!["segment1", "segment2", "segment3"]);
    }
}
