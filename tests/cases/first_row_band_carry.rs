//! 쪽 중간에서 시작한 표의 첫 행이 선언 빈 띠 행이면(글은 남은 자리에 다 들어가고 칸 선언 높이의 띠만 넘친다) 첫 행이라고
//! 통째로 두지 않는다 — 자르는 선에서 가르고 12.8pt 이상 남은 띠를 다음 쪽 첫머리로 넘긴다. 글 없는 끝 띠 조각을 버리는
//! 문턱도 같은 12.8pt 다(종전 25px 은 맥이 그리는 17~25px 꼬리를 버렸다).
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측 · 한/글 저장 사다리
//!
//! `samples/issue5699/37787_regulatory_impact.hwp`: 맥 49쪽(rhwp 종전 47쪽).
//! - 12쪽 끝 1×2 표(문단 143) — 선언 60.2px · 행 107.7px. 맥은 띠를 13쪽 머리로 넘겨 13쪽 첫 문단이 저장 3983HU 자리다.
//! - 13쪽 끝 1×2 표(문단 154) — 남은 띠 19.5px(14.6pt). 맥은 그 띠만 담은 14쪽(쪽 번호만 보이는 쪽)을 두고, 쪽 나누기 앞
//!   문단 155 «< 규제의 개요 >» 를 15쪽에서 연다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/issue5699/37787_regulatory_impact.hwp";

fn items(doc: &HwpDocument, page: u32) -> Vec<String> {
    doc.dump_page_items(Some(page))
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            t.starts_with("FullParagraph")
                || t.starts_with("PartialParagraph")
                || t.starts_with("Table")
                || t.starts_with("PartialTable")
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn a_mid_page_first_band_row_carries_its_tail_to_the_next_page() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    assert_eq!(doc.page_count(), 49, "맥 한글 49쪽");

    let p13 = items(&doc, 12);
    assert!(
        p13.first()
            .is_some_and(|l| l.contains("PartialTable   pi=143") && l.contains("cont=true")),
        "13쪽은 문단 143 표의 띠 이어진 조각으로 열려야 한다\n{p13:#?}"
    );
    let p14 = items(&doc, 13);
    assert!(
        p14.len() == 1 && p14[0].contains("PartialTable   pi=154") && p14[0].contains("cont=true"),
        "14쪽은 문단 154 표의 띠 조각만 담아야 한다(맥: 쪽 번호만 보이는 쪽)\n{p14:#?}"
    );
    let p15 = items(&doc, 14);
    assert!(
        p15.first()
            .is_some_and(|l| l.contains("FullParagraph  pi=155")),
        "15쪽은 쪽 나누기 앞 문단 155 로 열려야 한다\n{p15:#?}"
    );
}
