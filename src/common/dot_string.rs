use std::ops::{Index, Range, RangeFrom, RangeTo, RangeFull};

/// ドット区切り文字列へのインデックスアクセスを提供
///
/// # Examples
///
/// ```
/// use state_engine::common::DotString;
///
/// let dot_string = DotString::new("cache.user.org_id");
/// assert_eq!(&dot_string[0], "cache");
/// assert_eq!(&dot_string[1], "user");
/// assert_eq!(&dot_string[2], "org_id");
/// assert_eq!(dot_string.len(), 3);
/// ```
pub struct DotString {
    key: String,
    segments: Vec<String>,
}

impl DotString {
    /// 新しい DotString インスタンスを作成
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::DotString;
    ///
    /// let dot_string = DotString::new("cache.user.org_id");
    /// assert_eq!(dot_string.len(), 3);
    /// ```
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

    /// セグメント数を返す
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::DotString;
    ///
    /// let dot_string = DotString::new("cache.user.org_id");
    /// assert_eq!(dot_string.len(), 3);
    ///
    /// let empty = DotString::new("");
    /// assert_eq!(empty.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// 元のキー文字列を返す
    pub fn as_str(&self) -> &str {
        &self.key
    }

    /// セグメントのイテレータを返す
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.segments.iter().map(|s| s.as_str())
    }
}

// インデックスアクセス: dot_string[0]
impl Index<usize> for DotString {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        &self.segments[index]
    }
}

// 範囲アクセス: dot_string[0..2]
impl Index<Range<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// 範囲アクセス: dot_string[0..]
impl Index<RangeFrom<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: RangeFrom<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// 範囲アクセス: dot_string[..2]
impl Index<RangeTo<usize>> for DotString {
    type Output = [String];

    fn index(&self, range: RangeTo<usize>) -> &Self::Output {
        &self.segments[range]
    }
}

// 範囲アクセス: dot_string[..]
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
    fn test_new() {
        let dot_string = DotString::new("cache.user.org_id");
        assert_eq!(dot_string.len(), 3);
        assert_eq!(dot_string.as_str(), "cache.user.org_id");
    }

    #[test]
    fn test_empty() {
        let dot_string = DotString::new("");
        assert_eq!(dot_string.len(), 0);
    }

    #[test]
    fn test_single_segment() {
        let dot_string = DotString::new("cache");
        assert_eq!(dot_string.len(), 1);
        assert_eq!(&dot_string[0], "cache");
    }

    #[test]
    fn test_index_access() {
        let dot_string = DotString::new("cache.user.org_id");
        assert_eq!(&dot_string[0], "cache");
        assert_eq!(&dot_string[1], "user");
        assert_eq!(&dot_string[2], "org_id");
    }

    #[test]
    fn test_range_access() {
        let dot_string = DotString::new("cache.user.org_id");

        // 0..2
        assert_eq!(&dot_string[0..2], &["cache".to_string(), "user".to_string()]);

        // 1..
        assert_eq!(&dot_string[1..], &["user".to_string(), "org_id".to_string()]);

        // ..2
        assert_eq!(&dot_string[..2], &["cache".to_string(), "user".to_string()]);

        // ..
        assert_eq!(&dot_string[..], &["cache".to_string(), "user".to_string(), "org_id".to_string()]);
    }

    #[test]
    fn test_negative_index_emulation() {
        let dot_string = DotString::new("cache.user.org_id");

        // Python の string[-1] に相当: dot_string[len-1]
        let last = &dot_string[dot_string.len() - 1];
        assert_eq!(last, "org_id");

        // Python の string[:-1] に相当: dot_string[..len-1]
        let without_last = &dot_string[..dot_string.len() - 1];
        assert_eq!(without_last, &["cache".to_string(), "user".to_string()]);
    }


    #[test]
    fn test_iter() {
        let dot_string = DotString::new("cache.user.org_id");
        let collected: Vec<&str> = dot_string.iter().collect();
        assert_eq!(collected, vec!["cache", "user", "org_id"]);
    }
}
