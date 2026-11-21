//! Carets and selections (`JARAK-FR-012`).
//!
//! A [`Range`] has an anchor (where the selection started) and a head (where the caret is); an
//! empty range is a caret. A [`Selection`] holds one or more ranges — several for multiple carets
//! (Step 96) — kept sorted and never overlapping, with one primary range (the one the view follows).

/// One caret or selected range, in char positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub anchor: usize,
    pub head: usize,
    /// The display column vertical moves aim for (kept across short lines).
    pub goal: Option<usize>,
}

impl Range {
    pub fn new(anchor: usize, head: usize) -> Range {
        Range {
            anchor,
            head,
            goal: None,
        }
    }

    /// A caret at `pos`.
    pub fn point(pos: usize) -> Range {
        Range::new(pos, pos)
    }

    pub fn from(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn to(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn len(&self) -> usize {
        self.to() - self.from()
    }

    /// The head moved to `pos`; the anchor stays when `extend`, else follows.
    pub fn moved(self, pos: usize, extend: bool) -> Range {
        Range {
            anchor: if extend { self.anchor } else { pos },
            head: pos,
            goal: None,
        }
    }

    fn overlaps(&self, other: &Range) -> bool {
        // Two carets at the same place, or ranges sharing more than an edge.
        (self.is_empty() && other.is_empty() && self.head == other.head)
            || (self.from() < other.to() && other.from() < self.to())
            || (self.is_empty() && other.from() < self.head && self.head < other.to())
            || (other.is_empty() && self.from() < other.head && other.head < self.to())
    }
}

/// One or more ranges, sorted by position and never overlapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    ranges: Vec<Range>,
    primary: usize,
}

impl Selection {
    /// One range.
    pub fn single(r: Range) -> Selection {
        Selection {
            ranges: vec![r],
            primary: 0,
        }
    }

    /// One caret at `pos`.
    pub fn point(pos: usize) -> Selection {
        Selection::single(Range::point(pos))
    }

    /// Several ranges; `primary` indexes into `ranges` before sorting. Overlapping ranges merge.
    pub fn new(ranges: Vec<Range>, primary: usize) -> Selection {
        assert!(!ranges.is_empty(), "a selection has at least one range");
        let primary = primary.min(ranges.len() - 1);
        let mut s = Selection { ranges, primary };
        s.normalize();
        s
    }

    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    pub fn primary(&self) -> Range {
        self.ranges[self.primary]
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Every range changed by `f` (merging any that now overlap; the primary stays primary).
    pub fn map(&self, f: impl Fn(Range) -> Range) -> Selection {
        let ranges: Vec<Range> = self.ranges.iter().map(|r| f(*r)).collect();
        Selection::new(ranges, self.primary)
    }

    /// Adds a range and makes it primary.
    pub fn with(&self, r: Range) -> Selection {
        let mut ranges = self.ranges.clone();
        ranges.push(r);
        let n = ranges.len() - 1;
        Selection::new(ranges, n)
    }

    /// The last range alone (the caret after the last of several insertions).
    pub fn only_primary_last(&self) -> Selection {
        Selection::single(*self.ranges.last().unwrap_or(&Range::point(0)))
    }

    /// Only the primary range.
    pub fn only_primary(&self) -> Selection {
        Selection::single(self.primary())
    }

    /// Every position clamped to `len` (after the text shrank).
    pub fn clamped(&self, len: usize) -> Selection {
        self.map(|r| Range {
            anchor: r.anchor.min(len),
            head: r.head.min(len),
            goal: r.goal,
        })
    }

    fn normalize(&mut self) {
        let mut tagged: Vec<(Range, bool)> = self
            .ranges
            .iter()
            .enumerate()
            .map(|(i, r)| (*r, i == self.primary))
            .collect();
        tagged.sort_by_key(|(r, _)| (r.from(), r.to()));
        let mut out: Vec<(Range, bool)> = Vec::with_capacity(tagged.len());
        for (r, p) in tagged {
            if let Some((last, lp)) = out.last_mut()
                && last.overlaps(&r)
            {
                let from = last.from().min(r.from());
                let to = last.to().max(r.to());
                // Keep the merged range's direction from the one that reaches further.
                let forward = if r.to() >= last.to() {
                    r.head >= r.anchor
                } else {
                    last.head >= last.anchor
                };
                *last = if forward {
                    Range::new(from, to)
                } else {
                    Range::new(to, from)
                };
                *lp |= p;
                continue;
            }
            out.push((r, p));
        }
        self.primary = out.iter().position(|(_, p)| *p).unwrap_or(out.len() - 1);
        self.ranges = out.into_iter().map(|(r, _)| r).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_have_a_direction_and_a_caret_is_empty() {
        let r = Range::new(7, 3);
        assert_eq!((r.from(), r.to(), r.len()), (3, 7, 4));
        assert!(!r.is_empty() && Range::point(4).is_empty());
        assert_eq!(Range::point(4).moved(9, true), Range::new(4, 9));
        assert_eq!(Range::new(1, 4).moved(9, false), Range::point(9));
    }

    #[test]
    fn ranges_sort_merge_and_keep_the_primary() {
        let s = Selection::new(
            vec![
                Range::point(10),
                Range::new(2, 5),
                Range::new(4, 8),
                Range::point(10),
            ],
            0,
        );
        assert_eq!(s.ranges(), &[Range::new(2, 8), Range::point(10)]);
        assert_eq!(s.primary(), Range::point(10));
        // Ranges that only touch stay separate (two words selected side by side).
        let s = Selection::new(vec![Range::new(0, 3), Range::new(3, 6)], 1);
        assert_eq!(s.len(), 2);
        assert_eq!(s.primary(), Range::new(3, 6));
        // A caret inside a range merges into it.
        let s = Selection::new(vec![Range::new(0, 6), Range::point(3)], 1);
        assert_eq!(s.ranges(), &[Range::new(0, 6)]);
        assert_eq!(s.primary_index(), 0);
    }

    #[test]
    fn map_with_and_clamp() {
        let s = Selection::point(3).with(Range::point(8));
        assert_eq!(s.primary(), Range::point(8));
        let s = s.map(|r| r.moved(5, false));
        assert_eq!(s.ranges(), &[Range::point(5)], "carets meeting merge");
        let s = Selection::single(Range::new(2, 9)).clamped(4);
        assert_eq!(s.primary(), Range::new(2, 4));
    }
}
