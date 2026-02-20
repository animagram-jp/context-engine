use super::bi_map::BiMap;

pub struct Index {
    bi_map: BiMap,
}

impl Index {
    pub fn new() -> Self {
        Self {
            bi_map: BiMap::new(),
        }
    }

    /// Interns each segment of a dot-separated path and returns their indices.
    ///
    /// # Examples
    ///
    /// ```
    /// use state_engine::common::Index;
    ///
    /// let mut index = Index::new();
    ///
    /// let v0 = index.from_dot_string("foo.bar.baz");
    /// let v1 = index.from_dot_string("foo.bar.baz");
    ///
    /// // same path returns same indices
    /// assert_eq!(v0, v1);
    ///
    /// // shared segments share indices
    /// let v2 = index.from_dot_string("foo.bar.qux");
    /// assert_eq!(v2[0], v0[0]); // "foo"
    /// assert_eq!(v2[1], v0[1]); // "bar"
    /// assert_ne!(v2[2], v0[2]); // "qux" != "baz"
    /// ```
    pub fn from_dot_string(&mut self, path: &str) -> Vec<usize> {
        path.split('.').map(|seg| self.bi_map.set(seg.as_bytes().to_vec())).collect()
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segments_are_interned_across_paths() {
        let mut index = Index::new();
        let v0 = index.from_dot_string("a.b.c");
        let v1 = index.from_dot_string("a.b.d");

        assert_eq!(v0[0], v1[0]); // "a"
        assert_eq!(v0[1], v1[1]); // "b"
        assert_ne!(v0[2], v1[2]); // "c" != "d"
    }
}
