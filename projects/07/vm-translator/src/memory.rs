use crate::parser::Segment;

/// Segment access pattern - makes impossible states unrepresentable
/// Eliminates need for .expect() calls by encoding segment types in the type system
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentAccess {
    /// Direct addressing: immediate value or fixed RAM address
    Direct { addr: u16 },
    /// Indirect addressing: base pointer + index
    Indirect { base: &'static str },
    /// Static addressing: filename.index
    Static { index: u16 },
}

/// Determine the segment access pattern for a given segment and index
/// Type-safe: returns explicit access pattern, eliminating panic potential
pub fn segment_access(segment: Segment, index: u16) -> SegmentAccess {
    match segment {
        Segment::Constant => SegmentAccess::Direct { addr: index },
        Segment::Local => SegmentAccess::Indirect { base: "LCL" },
        Segment::Argument => SegmentAccess::Indirect { base: "ARG" },
        Segment::This => SegmentAccess::Indirect { base: "THIS" },
        Segment::That => SegmentAccess::Indirect { base: "THAT" },
        Segment::Pointer => SegmentAccess::Direct { addr: 3 + index },
        Segment::Temp => SegmentAccess::Direct { addr: 5 + index },
        Segment::Static => SegmentAccess::Static { index },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_access_constant() {
        assert_eq!(
            segment_access(Segment::Constant, 42),
            SegmentAccess::Direct { addr: 42 }
        );
    }

    #[test]
    fn test_segment_access_indirect() {
        assert_eq!(
            segment_access(Segment::Local, 0),
            SegmentAccess::Indirect { base: "LCL" }
        );
        assert_eq!(
            segment_access(Segment::Argument, 0),
            SegmentAccess::Indirect { base: "ARG" }
        );
        assert_eq!(
            segment_access(Segment::This, 0),
            SegmentAccess::Indirect { base: "THIS" }
        );
        assert_eq!(
            segment_access(Segment::That, 0),
            SegmentAccess::Indirect { base: "THAT" }
        );
    }

    #[test]
    fn test_segment_access_pointer() {
        assert_eq!(
            segment_access(Segment::Pointer, 0),
            SegmentAccess::Direct { addr: 3 }
        );
        assert_eq!(
            segment_access(Segment::Pointer, 1),
            SegmentAccess::Direct { addr: 4 }
        );
    }

    #[test]
    fn test_segment_access_temp() {
        assert_eq!(
            segment_access(Segment::Temp, 0),
            SegmentAccess::Direct { addr: 5 }
        );
        assert_eq!(
            segment_access(Segment::Temp, 7),
            SegmentAccess::Direct { addr: 12 }
        );
    }

    #[test]
    fn test_segment_access_static() {
        assert_eq!(
            segment_access(Segment::Static, 5),
            SegmentAccess::Static { index: 5 }
        );
    }
}
