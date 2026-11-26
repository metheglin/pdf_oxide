//! Pages whose dictionaries sit in one object stream that decodes to more
//! than 1 MiB. Such a stream is kept decoded and its members are parsed one at
//! a time, so each page must still come back as its own dictionary.

use pdf_oxide::editor::DocumentEditor;
use pdf_oxide::PdfDocument;

const ROTATIONS: [i32; 4] = [0, 90, 180, 270];

/// Four pages rotated 0, 90, 180 and 270 degrees, each showing "Page N". The
/// page dictionaries carry 400 KB of padding each and share one uncompressed
/// object stream.
fn large_object_stream_pdf() -> Vec<u8> {
    let pages = ROTATIONS.len();
    let font = 3;
    let first_content = 4;
    let first_page = first_content + pages;
    let objstm = first_page + pages;
    let xref = objstm + 1;

    let mut direct: Vec<(usize, Vec<u8>)> = vec![
        (1, b"<</Type/Catalog/Pages 2 0 R>>".to_vec()),
        (
            2,
            format!(
                "<</Type/Pages/Count {}/Kids[{}]>>",
                pages,
                (0..pages)
                    .map(|i| format!("{} 0 R", first_page + i))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .into_bytes(),
        ),
        (
            font,
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/Encoding/WinAnsiEncoding>>".to_vec(),
        ),
    ];
    for i in 0..pages {
        let content = format!("BT /F1 24 Tf 72 400 Td (Page {}) Tj ET", i + 1);
        direct.push((
            first_content + i,
            format!("<</Length {}>>\nstream\n{}\nendstream", content.len(), content).into_bytes(),
        ));
    }

    let padding = "x".repeat(400 * 1024);
    let bodies: Vec<String> = (0..pages)
        .map(|i| {
            format!(
                "<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Rotate {}\
                 /Resources<</Font<</F1 {} 0 R>>>>/Contents {} 0 R/Pad({})>>",
                ROTATIONS[i],
                font,
                first_content + i,
                padding
            )
        })
        .collect();
    let mut index = String::new();
    let mut offset = 0;
    for (i, body) in bodies.iter().enumerate() {
        index.push_str(&format!("{} {} ", first_page + i, offset));
        offset += body.len() + 1;
    }
    let stream = format!("{}{}", index, bodies.join("\n"));
    assert!(stream.len() > 1024 * 1024);

    let mut out = b"%PDF-1.6\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![0usize; xref + 1];
    for (num, body) in &direct {
        offsets[*num] = out.len();
        out.extend_from_slice(format!("{} 0 obj\n", num).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    offsets[objstm] = out.len();
    out.extend_from_slice(
        format!(
            "{} 0 obj\n<</Type/ObjStm/N {}/First {}/Length {}>>\nstream\n",
            objstm,
            pages,
            index.len(),
            stream.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(stream.as_bytes());
    out.extend_from_slice(b"\nendstream\nendobj\n");

    // Cross-reference stream, /W [1 4 2]: type, offset or stream number, index.
    let xref_offset = out.len();
    offsets[xref] = xref_offset;
    let mut table = vec![0u8, 0, 0, 0, 0, 0xff, 0xff];
    for num in 1..=xref {
        if (first_page..objstm).contains(&num) {
            table.push(2);
            table.extend_from_slice(&(objstm as u32).to_be_bytes());
            table.extend_from_slice(&((num - first_page) as u16).to_be_bytes());
        } else {
            table.push(1);
            table.extend_from_slice(&(offsets[num] as u32).to_be_bytes());
            table.extend_from_slice(&[0, 0]);
        }
    }
    out.extend_from_slice(
        format!(
            "{} 0 obj\n<</Type/XRef/Size {}/W[1 4 2]/Root 1 0 R/Length {}>>\nstream\n",
            xref,
            xref + 1,
            table.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&table);
    out.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{}\n%%EOF\n", xref_offset).as_bytes(),
    );
    out
}

#[test]
fn each_page_of_a_large_object_stream_is_read_as_itself() {
    let doc = PdfDocument::from_bytes(large_object_stream_pdf()).unwrap();
    assert_eq!(doc.page_count().unwrap(), ROTATIONS.len());

    // Backwards first, then forwards, so no page is reached in stream order
    // only, and every page is read more than once.
    for i in (0..ROTATIONS.len()).rev().chain(0..ROTATIONS.len()) {
        assert_eq!(doc.get_page_rotation(i).unwrap(), ROTATIONS[i], "page {}", i);
        let text = doc.extract_text(i).unwrap();
        assert!(text.contains(&format!("Page {}", i + 1)), "page {}: {:?}", i, text);
    }
}

#[test]
fn editor_reads_page_rotation_from_a_large_object_stream() {
    let doc = PdfDocument::from_bytes(large_object_stream_pdf()).unwrap();
    let mut editor = DocumentEditor::from_document(doc).unwrap();

    for _ in 0..2 {
        for (i, expected) in ROTATIONS.iter().enumerate() {
            assert_eq!(editor.get_page_rotation(i).unwrap(), *expected, "page {}", i);
        }
    }

    // A staged rotation is still what the editor reports.
    editor.set_page_rotation(1, 270).unwrap();
    assert_eq!(editor.get_page_rotation(1).unwrap(), 270);
    assert_eq!(editor.get_page_rotation(2).unwrap(), 180);
}
