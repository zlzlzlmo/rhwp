//! 쪽 중간의 RowBreak 병합 묶음이 자르는 선(본문 아래 − 바깥 아래 여백 − 100HU)을 넘는데, 그 선이 걸린 행까지 시작한
//! 칸의 글이 모두 선 위에 들면 묶음을 그 선에서 끊고 그 행의 남은 빈 띠만 다음 쪽 첫머리에 그린다(행 하나의 띠 넘김과
//! 같은 연산).
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! `samples/webiz-upload/cdd207a7_expo_application.hwp`(베트남 국제 산업기계 박람회 참가신청서, 공고 첨부 빈 양식):
//! 21×7 표의 15~17행 묶음 — «제품» 15~16행 · «제품 및 서비스 소개» 16~17행(안내문 «※ 전시 제품·서비스의 개요…»)이
//! 계단식으로 겹친다. 맥은 1쪽에 소개 칸 글까지 그리고 표 바닥이 768.5pt(자르는 선), 2쪽 머리 띠는 27.7pt 다.
//! rhwp 종전: 조각 끝 행이 소개 칸 병합 끝보다 앞이라 렌더가 그 칸을 안 그리고 이어진 조각은 소비됐다고 봐 글이 사라졌다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/webiz-upload/cdd207a7_expo_application.hwp";

fn items(doc: &HwpDocument, page: u32) -> Vec<String> {
    doc.dump_page_items(Some(page))
        .lines()
        .filter(|line| line.trim_start().starts_with("PartialTable"))
        .map(str::to_string)
        .collect()
}

#[test]
fn a_staggered_rowspan_block_keeps_its_text_on_the_first_page_and_carries_the_band() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    assert_eq!(doc.page_count(), 3, "맥 한글 3쪽");

    let svg = doc.render_page_svg(0).expect("1쪽");
    let marks = svg.matches(">※<").count();
    assert_eq!(
        marks, 3,
        "1쪽에 안내문 «※» 셋(외국어 카탈로그 · 인원수 · 전시 제품 소개)이 그려져야 한다"
    );

    let p1 = items(&doc, 0);
    assert!(
        p1.iter()
            .any(|l| l.contains("pi=2") && l.contains("rows=0..17")),
        "1쪽 조각은 소개 칸이 시작한 16행까지다\n{p1:#?}"
    );
    let p2 = items(&doc, 1);
    assert!(
        p2.first().is_some_and(|l| l.contains("pi=2")
            && l.contains("rows=16..21")
            && l.contains("cont=true")),
        "2쪽은 16행의 남은 띠부터 잇는다\n{p2:#?}"
    );
}
