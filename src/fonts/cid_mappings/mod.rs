//! CID to Unicode mappings for predefined Adobe character collections.
//!
//! This module provides CID (Character Identifier) to Unicode mappings for the
//! standard Adobe CJK character collections used in PDF documents.
//!
//! Per PDF Spec ISO 32000-1:2008 Section 9.7.5.2, these predefined CMaps map
//! CIDs from specific character collections to Unicode code points.
//!
//! # Supported Character Collections
//!
//! - **Adobe-GB1**: Simplified Chinese (GB 2312 + extensions)
//! - **Adobe-Japan1**: Japanese (JIS X 0208, JIS X 0212)
//! - **Adobe-CNS1**: Traditional Chinese (CNS 11643)
//! - **Adobe-Korea1**: Korean (KS X 1001)
//!
//! # References
//!
//! - Adobe Technical Note #5078: Adobe-Japan1-7
//! - Adobe Technical Note #5079: Adobe-GB1-5
//! - Adobe Technical Note #5080: Adobe-CNS1-7
//! - Adobe Technical Note #5093: Adobe-Korea1-2
//!
//! # Implementation Notes
//!
//! This module uses `phf_map!` for O(1) CID-to-Unicode lookup, following the
//! same pattern as `adobe_glyph_list.rs`.

mod adobe_arabic;
mod adobe_cns1;
mod adobe_gb1;
mod adobe_japan1;
mod adobe_korea1;

/// Look up Unicode code point for a CID in Adobe-GB1 (Simplified Chinese).
///
/// This mapping corresponds to the UniGB-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-29063 for Adobe-GB1-5)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_gb1(cid: u16) -> Option<u32> {
    adobe_gb1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-Japan1 (Japanese).
///
/// This mapping corresponds to the UniJIS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-23057 for Adobe-Japan1-7)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_japan1(cid: u16) -> Option<u32> {
    adobe_japan1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-CNS1 (Traditional Chinese).
///
/// This mapping corresponds to the UniCNS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-19155 for Adobe-CNS1-7)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_cns1(cid: u16) -> Option<u32> {
    adobe_cns1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-Korea1 (Korean).
///
/// This mapping corresponds to the UniKS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-18351 for Adobe-Korea1-2)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_korea1(cid: u16) -> Option<u32> {
    adobe_korea1::lookup(cid)
}

/// look up Unicode code point for a CID in
/// Adobe-Arabic-1 / Adobe-Persian-1 (used by Persian / Farsi /
/// Pashto / Urdu fonts that ship without ToUnicode CMaps).
///
/// Stub implementation: identity mapping for the Arabic block
/// (U+0600–U+06FF) and Arabic Presentation Forms (U+FB50–U+FDFF +
/// U+FE70–U+FEFF). The official Adobe-Arabic-1-UCS2 CMap is
/// follow-up work.
///
/// Returns the Unicode code point, or `None` if the CID isn't in
/// the supported range (caller falls back to the existing chain).
///
/// # Arguments
///
/// * `cid` - Character Identifier
#[inline]
pub fn lookup_adobe_arabic(cid: u16) -> Option<u32> {
    adobe_arabic::lookup(cid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adobe_gb1_ascii_from_cid() {
        // CID 34 maps to 'A' (U+0041), CID 91 maps to 'z' (U+007A)
        // Note: CIDs are indices in the character collection, not Unicode values
        assert_eq!(lookup_adobe_gb1(34), Some(0x41)); // CID 34 -> A
        assert_eq!(lookup_adobe_gb1(91), Some(0x7A)); // CID 91 -> z
    }

    #[test]
    fn test_adobe_japan1_ascii_from_cid() {
        // CID 34 maps to 'A' (U+0041)
        assert_eq!(lookup_adobe_japan1(34), Some(0x41)); // CID 34 -> A
        assert_eq!(lookup_adobe_japan1(91), Some(0x7A)); // CID 91 -> z
    }

    #[test]
    fn test_adobe_japan1_half_width_latin_and_digits() {
        // CIDs 231..=325 are the half-width forms of the ASCII range; a
        // non-embedded Japanese font under Identity-H shows Latin text and
        // digits with them. They were absent, so the CID was read as a code
        // point ("C" came out as U+010A).
        assert_eq!(lookup_adobe_japan1(231), Some(0x2002)); // half-width space: EN SPACE
        assert_eq!(lookup_adobe_japan1(247), Some(0x30)); // 0
        assert_eq!(lookup_adobe_japan1(250), Some(0x33)); // 3
        assert_eq!(lookup_adobe_japan1(266), Some(0x43)); // C
        assert_eq!(lookup_adobe_japan1(310), Some(0x6F)); // o
        for cid in 231..=632u16 {
            assert!(lookup_adobe_japan1(cid).is_some(), "CID {cid} has no mapping");
        }
    }

    #[test]
    fn test_adobe_japan1_ideographs_are_not_radical_forms() {
        // The table took the first code Adobe lists for a CID, which for these
        // is a radical form (U+2E80..=U+2FDF) no CJK text font is sure to have.
        assert_eq!(lookup_adobe_japan1(2260), Some(0x793A)); // 示, was U+2EAC
        assert_eq!(lookup_adobe_japan1(2664), Some(0x9752)); // 青, was U+2ED8
        assert_eq!(lookup_adobe_japan1(1200), Some(0x4E00)); // 一, was U+2F00
        // The few CIDs that really are radical glyphs keep their code.
        let radicals = (1..=23057u16)
            .filter(|&cid| matches!(lookup_adobe_japan1(cid), Some(0x2E80..=0x2FDF)))
            .count();
        assert_eq!(radicals, 9);
    }

    #[test]
    fn test_adobe_japan1_variant_glyph_maps_to_its_base_character() {
        // CID 230 is the slashed zero: Adobe gives U+0030 U+FE00.
        assert_eq!(lookup_adobe_japan1(230), Some(0x30));
        // Entries that were already there are untouched (CID 95 stays "~").
        assert_eq!(lookup_adobe_japan1(95), Some(0x7E));
    }

    #[test]
    fn test_adobe_japan1_hiragana() {
        // Test Hiragana from CID
        assert_eq!(lookup_adobe_japan1(843), Some(0x3042)); // CID 843 -> あ
    }

    #[test]
    fn test_adobe_cns1_ascii_from_cid() {
        // CID 34 maps to 'A' (U+0041)
        assert_eq!(lookup_adobe_cns1(34), Some(0x41)); // CID 34 -> A
    }

    #[test]
    fn test_adobe_korea1_ascii_from_cid() {
        // CID 34 maps to 'A' (U+0041)
        assert_eq!(lookup_adobe_korea1(34), Some(0x41)); // CID 34 -> A
    }

    #[test]
    fn test_adobe_korea1_hangul() {
        // Test Hangul syllable from CID (from Adobe cid2code.txt)
        assert_eq!(lookup_adobe_korea1(1086), Some(0xAC00)); // CID 1086 -> 가
    }
}
