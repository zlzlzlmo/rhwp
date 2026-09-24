//! RowBreak table fragments must not keep an intra-row slice when that slice
//! makes the whole fragment exceed the page.
//!
//! Regression sample: `samples/kps-ai.hwp`, page 37. Paragraph 329 is a large
//! 32x2 RowBreak table. Keeping a tiny slice of the next row overflows the page
//! and should be deferred to page 38.
//!
//! 맥 한글 12.30(`up-kpsai.pdf`) 37쪽 마지막 행 경계는 763.1pt 로 행 0..15 에서 끝나고 38쪽이 행 15 부터
//! 잇는다. 종전 핀(0..16)은 비정상 세로 여백 칸(여백 합 ≥ 칸 선언)을 줄인 여백의 컷 높이로 들여 한 행을
//! 더 넣던 값이다 — 그 행은 레이아웃이 측정 높이(원 여백)로 그려 쪽 바닥 787.5pt 까지 내려갔다.

use std::fs;
use std::path::Path;

fn load_doc(rel_path: &str) -> rhwp::wasm_api::HwpDocument {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(repo_root).join(rel_path);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
    rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
        .unwrap_or_else(|e| panic!("parse {rel_path}: {e:?}"))
}

fn page_dump(rel_path: &str, page_idx: u32) -> String {
    let doc = load_doc(rel_path);
    doc.dump_page_items(Some(page_idx))
}

#[test]
fn kps_ai_page37_defers_overflowing_split_row_slice() {
    let sample = "samples/kps-ai.hwp";

    let page37 = page_dump(sample, 36);
    assert!(
        page37.contains("PartialTable   pi=329 ci=0  rows=0..15"),
        "page 37 should end at the last fully fitting row:\n{page37}"
    );
    assert!(
        !page37.contains("end_cut="),
        "page 37 must not keep an overflowing row slice:\n{page37}"
    );

    let page38 = page_dump(sample, 37);
    assert!(
        page38.contains("PartialTable   pi=329 ci=0  rows=15..32  cont=true"),
        "page 38 should continue from row 15:\n{page38}"
    );
}

#[test]
fn synam_001_page14_uses_row_budget_after_repeated_header() {
    let sample = "samples/synam-001.hwp";

    let page13 = page_dump(sample, 12);
    assert!(
        page13.contains("PartialTable   pi=140 ci=0  rows=2..7"),
        "page 13 should include the first visible slice of row 6 after repeated header rows:\n{page13}"
    );
    assert!(
        page13.contains("start_cut=[2, 19] end_cut=[2, 13]"),
        "page 13 should continue row 2 and cut into row 6 instead of deferring row 6:\n{page13}"
    );

    let page14 = page_dump(sample, 13);
    assert!(
        page14.contains("start_cut=[2, 13]"),
        "page 14 should continue from the row 6 cut already started on page 13:\n{page14}"
    );
}

#[test]
fn synam_001_page5_splits_large_rowspan_block_like_hancom() {
    let sample = "samples/synam-001.hwp";
    let doc = load_doc(sample);
    assert_eq!(
        doc.page_count(),
        35,
        "synam-001 should match the Hancom PDF page count"
    );

    let page5 = doc.dump_page_items(Some(4));
    assert!(
        page5.contains("PartialTable   pi=69 ci=0  rows=0..5"),
        "page 5 should start the financial-assets row instead of deferring it:\n{page5}"
    );
    assert!(
        page5.contains("end_cut=[2, 2]"),
        "page 5 should keep the first visible slice of row 4:\n{page5}"
    );

    let page6 = doc.dump_page_items(Some(5));
    assert!(
        page6.contains("start_cut=[2, 2]"),
        "page 6 should continue the row 4 split from page 5:\n{page6}"
    );
    assert!(
        page6.contains("FullParagraph  pi=72"),
        "page 6 should also contain the illegal-transfer body paragraph:\n{page6}"
    );

    let page7 = doc.dump_page_items(Some(6));
    assert!(
        page7.contains("Table          pi=76"),
        "page 7 should start at section 4 after the rowbreak table and illegal-transfer text:\n{page7}"
    );
}
