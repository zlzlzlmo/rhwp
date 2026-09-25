//! 단 첫 항목이 글 없는 host 의 문단 기준 자리차지 표(비 TAC)면 조판도 레이아웃처럼 그 표 문단의 저장 첫 줄을 쪽
//! 기준으로 둔다. 기준 없이 다음 문단에서 lazy 로 역산하면 끝 줄 간격 다리 몫만큼 기준이 어긋나, 합성 줄의 낡은
//! vpos 를 «구조적 재앵커»로 믿고 크게 뛴다 — 레이아웃은 같은 자리를 합성 소폭 전진(+19px)으로 보고 무시한다.
//!
//! ## 형상 — 위비즈가 채운 도약 제출본(`samples/webiz-fill/leap_viz0_filled.hwp`)
//!
//! 7쪽은 9×3 표(문단 55)로 시작한다. 채움이 재조판 뒤 일부 문단만 되돌려 합성 사다리가 엇갈린 문서라, 종전 조판은
//! 문단 57 을 195.9 → 400.1px 로 204px 뛰어 7쪽을 603pt 에서 끊고 8쪽을 «4-1-2» 제목으로 열었다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! 맥은 7쪽을 «4-1-2» 아래 인력 표(문단 71)의 첫 행까지 채우고(끝 줄 756pt «공동대표 포함)»), 8쪽을 그 표의 이어진
//! 조각(반복 제목 행 «고용여부 순번 직위 …»)으로 연다. 쪽 수 9 는 같다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/webiz-fill/leap_viz0_filled.hwp";

#[test]
fn a_page_opened_by_an_empty_host_table_keeps_the_layout_page_origin() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    assert_eq!(doc.page_count(), 9, "맥 한글 9쪽");

    let p7 = doc.dump_page_items(Some(6));
    let last = p7
        .lines()
        .filter(|line| line.trim_start().starts_with(['F', 'P', 'T']))
        .last()
        .expect("7쪽 항목");
    assert!(
        last.contains("PartialTable   pi=71")
            && last.contains("rows=0..1")
            && last.contains("cont=false"),
        "7쪽은 인력 표(문단 71)의 첫 행까지 채워야 한다(맥 끝 줄 756pt)\n{p7}"
    );
    let p8 = doc.dump_page_items(Some(7));
    let first = p8
        .lines()
        .find(|line| line.trim_start().starts_with(['F', 'P', 'T']))
        .expect("8쪽 항목");
    assert!(
        first.contains("PartialTable   pi=71") && first.contains("cont=true"),
        "8쪽은 인력 표의 이어진 조각으로 열려야 한다\n{p8}"
    );
}
