//! Memory segment address calculation.
//!
//! Handles the mapping between VM segments and Hack RAM addresses.

use crate::parser::Segment;

/// Segment access mode for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentAccess {
    /// Constant values (immediate)
    Constant,
    /// Indirect via base pointer (LCL, ARG, THIS, THAT)
    Indirect(&'static str),
    /// Direct RAM address (temp, pointer)
    Direct,
    /// Static variables with filename prefix
    Static,
}

/// Get the base pointer symbol for indirect segments.
pub fn segment_base_symbol(segment: Segment) -> Option<&'static str> {
    match segment {
        Segment::Local => Some("LCL"),
        Segment::Argument => Some("ARG"),
        Segment::This => Some("THIS"),
        Segment::That => Some("THAT"),
        _ => None,
    }
}

/// Determine the access mode for a segment.
pub fn segment_access(segment: Segment) -> SegmentAccess {
    match segment {
        Segment::Constant => SegmentAccess::Constant,
        Segment::Local => SegmentAccess::Indirect("LCL"),
        Segment::Argument => SegmentAccess::Indirect("ARG"),
        Segment::This => SegmentAccess::Indirect("THIS"),
        Segment::That => SegmentAccess::Indirect("THAT"),
        Segment::Pointer | Segment::Temp => SegmentAccess::Direct,
        Segment::Static => SegmentAccess::Static,
    }
}

/// Calculate the RAM address for temp segment.
/// Temp segment is RAM[5..12], so temp i maps to RAM[5+i].
#[inline]
pub fn temp_address(index: u16) -> u16 {
    5 + index
}

/// Get the symbol for pointer segment.
/// pointer 0 = THIS (RAM[3])
/// pointer 1 = THAT (RAM[4])
#[inline]
pub fn pointer_symbol(index: u16) -> &'static str {
    if index == 0 { "THIS" } else { "THAT" }
}

/// Check if a segment uses indirect addressing.
#[inline]
pub fn is_indirect_segment(segment: Segment) -> bool {
    matches!(
        segment,
        Segment::Local | Segment::Argument | Segment::This | Segment::That
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_base_symbol() {
        assert_eq!(segment_base_symbol(Segment::Local), Some("LCL"));
        assert_eq!(segment_base_symbol(Segment::Argument), Some("ARG"));
        assert_eq!(segment_base_symbol(Segment::This), Some("THIS"));
        assert_eq!(segment_base_symbol(Segment::That), Some("THAT"));
        assert_eq!(segment_base_symbol(Segment::Constant), None);
        assert_eq!(segment_base_symbol(Segment::Temp), None);
    }

    #[test]
    fn test_temp_address() {
        assert_eq!(temp_address(0), 5);
        assert_eq!(temp_address(3), 8);
        assert_eq!(temp_address(7), 12);
    }

    #[test]
    fn test_pointer_symbol() {
        assert_eq!(pointer_symbol(0), "THIS");
        assert_eq!(pointer_symbol(1), "THAT");
    }

    #[test]
    fn test_segment_access() {
        assert_eq!(segment_access(Segment::Constant), SegmentAccess::Constant);
        assert_eq!(
            segment_access(Segment::Local),
            SegmentAccess::Indirect("LCL")
        );
        assert_eq!(segment_access(Segment::Temp), SegmentAccess::Direct);
        assert_eq!(segment_access(Segment::Static), SegmentAccess::Static);
    }

    #[test]
    fn test_is_indirect_segment() {
        assert!(is_indirect_segment(Segment::Local));
        assert!(is_indirect_segment(Segment::Argument));
        assert!(!is_indirect_segment(Segment::Constant));
        assert!(!is_indirect_segment(Segment::Temp));
    }
}
