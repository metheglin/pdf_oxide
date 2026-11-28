//! Images inside Form XObjects. The per-form image cache is bounded by the
//! size of the decoded data, so a form whose images exceed the budget is
//! extracted again on each use; the result must be the same as from the cache.

use std::io::Write;

use pdf_oxide::extractors::{ImageData, PdfImage};
use pdf_oxide::PdfDocument;

const BIG_W: u32 = 5000;
const BIG_H: u32 = 4500; // 5000 x 4500 RGB = 67.5 MB, over the 64 MiB budget

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<<{}/Length {}>>\nstream\n", dict, data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// Two pages. Each paints form 5 (a 16 x 16 gray image) and form 6 (a
/// 5000 x 4500 RGB image), at different places on the two pages.
fn pdf() -> Vec<u8> {
    let small = deflate(&[0x80u8; 16 * 16]);
    let big = deflate(&vec![0x40u8; (BIG_W * BIG_H * 3) as usize]);
    let form = |image: u32| {
        stream(
            &format!(
                "/Type/XObject/Subtype/Form/BBox[0 0 1 1]/Resources<</XObject<</Im {} 0 R>>>>",
                image
            ),
            b"/Im Do",
        )
    };
    let page = |content: u32| {
        format!(
            "<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]\
             /Resources<</XObject<</Fs 5 0 R/Fb 6 0 R>>>>/Contents {} 0 R>>",
            content
        )
        .into_bytes()
    };
    let objects: Vec<Vec<u8>> = vec![
        b"<</Type/Catalog/Pages 2 0 R>>".to_vec(),
        b"<</Type/Pages/Count 2/Kids[3 0 R 4 0 R]>>".to_vec(),
        page(9),
        page(10),
        form(7),
        form(8),
        stream(
            "/Type/XObject/Subtype/Image/Width 16/Height 16/ColorSpace/DeviceGray\
             /BitsPerComponent 8/Filter/FlateDecode",
            &small,
        ),
        stream(
            &format!(
                "/Type/XObject/Subtype/Image/Width {}/Height {}/ColorSpace/DeviceRGB\
                 /BitsPerComponent 8/Filter/FlateDecode",
                BIG_W, BIG_H
            ),
            &big,
        ),
        stream("", b"q 100 0 0 100 50 50 cm /Fs Do Q q 200 0 0 180 300 400 cm /Fb Do Q"),
        stream("", b"q 50 0 0 50 10 20 cm /Fs Do Q q 400 0 0 360 100 300 cm /Fb Do Q"),
    ];

    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref
        )
        .as_bytes(),
    );
    out
}

fn summary(images: &[PdfImage]) -> Vec<(u32, u32, [i64; 4], usize)> {
    images
        .iter()
        .map(|img| {
            let b = img.bbox().expect("bbox");
            let len = match img.data() {
                ImageData::Jpeg(d) => d.len(),
                ImageData::Raw { pixels, .. } => pixels.len(),
            };
            let r = |v: f32| (v * 100.0).round() as i64;
            (img.width(), img.height(), [r(b.x), r(b.y), r(b.width), r(b.height)], len)
        })
        .collect()
}

#[test]
fn form_images_are_the_same_with_and_without_the_cache() {
    let doc = PdfDocument::from_bytes(pdf()).unwrap();

    let first = summary(&doc.extract_images(0).unwrap());
    assert_eq!(
        first,
        vec![
            (16, 16, [5000, 5000, 10000, 10000], 16 * 16),
            (BIG_W, BIG_H, [30000, 40000, 20000, 18000], (BIG_W * BIG_H * 3) as usize),
        ]
    );
    // Second call on the same page: the small form comes from the cache, the
    // big one (over the budget, so never cached) is extracted again.
    assert_eq!(summary(&doc.extract_images(0).unwrap()), first);

    // The same forms painted elsewhere on another page.
    let other = summary(&doc.extract_images(1).unwrap());
    assert_eq!(
        other,
        vec![
            (16, 16, [1000, 2000, 5000, 5000], 16 * 16),
            (BIG_W, BIG_H, [10000, 30000, 40000, 36000], (BIG_W * BIG_H * 3) as usize),
        ]
    );
    assert_eq!(summary(&doc.extract_images(0).unwrap()), first);
}
