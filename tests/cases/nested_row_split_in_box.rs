//! 쪽을 넘는 테두리 상자(1×1 · 셀 단위로 나눔) 안의 자리차지 중첩 표 — 맥 한글 12.30 이 두 가지를 다르게 놓는다.
//!
//! ## 형상 — 위비즈가 채운 서초AICT 데모데이 [붙임2] 제출본(`samples/webiz-fill/seocho_nested_row_split.hwpx`)
//!
//! 문단 1 ci1 = 자리차지 테두리 표(1×1 · 선언 66339HU · 쪽 나눔 «셀 단위로 나눔»). 칸 하나에 문단 15개: 빈 문단 둘(줄 7.6pt ·
//! 17.6pt) 뒤에 자리차지 9×4 중첩 표(`vertOffset` 285 · 바깥 위 여백 283 · 행마다 «셀 단위로 나눔»), 그 뒤 서명 문단들.
//! 중첩 표 행은 채움으로 글이 늘어 선언 높이(`cellSz`)보다 내용이 2~9배 크다(행5: 선언 52.2px · 내용 489.7px).
//! 표본은 사용자 글의 한글을 «가»로 바꾸고(글줄 폭 그대로) 그림을 같은 크기 회색으로 바꾸고 글 앞 도장 그림을 걷은
//! 축소본이다(도장은 글 앞 배치라 흐름 불변 — 걷기 전 축소본의 맥 PDF 가 원본과 좌표까지 같았다).
//!
//! ## 맥 한글 12.30 실측(같은 문서의 원본 PDF · 축소본 PDF 가 좌표까지 같다)
//!
//! 1. 중첩 표는 앞 빈 문단 둘의 줄 자리(25.2pt) **아래**에 선다 — 1쪽 상자 윗변 158.3pt · 중첩 표 윗변 193.3pt(35.0pt 아래
//!    = 칸 위 안 여백 1.4 + 빈 문단 25.2 + 세로 오프셋·바깥 위 여백 5.7 + 가운데 정렬 몫 2.7). 종전 rhwp 는 표를 칸 맨 위
//!    (4.2pt 아래)에 그려 빈 문단 자리만큼 위로 올렸다.
//! 2. 중첩 표 행5(6행째, 내용 367.6pt)는 1쪽 바닥에서 **줄 단위로** 끊겨 1쪽 189.0pt · 2쪽 178.6pt 로 나뉜다(선언 높이를
//!    내용과 같게 고친 표본도 같다). 종전 rhwp 는 행을 통째 2쪽으로 넘겨 1쪽 절반을 비웠다(중첩 표 아래 540.6pt · 맥 760.3pt).
//!    한/글이 저장한 표(선언 ≈ 내용)의 중첩 행은 그대로 통째다 — #6923 · #5908 표본의 쪽수를 지킨다.
//!
//! 남은 차이(이 시험이 잠그지 않는다): 중첩 표 윗변이 맥보다 5.5pt 위(바깥 위 여백·가운데 정렬 몫) · 이어진 쪽의 중첩 표
//! 바깥 위 여백 2.8pt.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/webiz-fill/seocho_nested_row_split.hwpx";

fn pt(px: f64) -> f64 {
    px * 72.0 / 96.0
}

fn tables_preorder<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    if matches!(node.node_type, RenderNodeType::Table(_)) {
        out.push(node);
    }
    for child in &node.children {
        tables_preorder(child, out);
    }
}

/// 쪽 하나의 (테두리 상자, 그 안 중첩 표) — 트리 순서로 테두리 상자 다음 표.
fn box_and_nested(doc: &HwpDocument, page: u32) -> (RenderNode, Option<RenderNode>) {
    let root = doc.build_page_render_tree(page).expect("쪽 렌더").root;
    let mut tables = Vec::new();
    tables_preorder(&root, &mut tables);
    let frame = tables
        .iter()
        .position(|t| matches!(&t.node_type, RenderNodeType::Table(n) if n.para_index == Some(1) && n.control_index == Some(1)))
        .expect("테두리 상자(문단 1 ci1)");
    let nested = tables.get(frame + 1).map(|t| (*t).clone());
    (tables[frame].clone(), nested)
}

fn doc() -> HwpDocument {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    HwpDocument::from_bytes(&std::fs::read(path).expect("표본")).expect("문서 로드")
}

#[test]
fn the_nested_table_stands_below_the_blank_paragraphs_of_the_box_cell() {
    let doc = doc();
    let (frame, nested) = box_and_nested(&doc, 0);
    let nested = nested.expect("1쪽 중첩 표");
    let gap = pt(nested.bbox.y - frame.bbox.y);
    // 맥 35.0pt. 빈 문단 둘(25.2pt)을 빼먹으면 4.2pt 다. 남은 5.5pt 차(바깥 위 여백 · 가운데 정렬 몫)는 잠그지 않는다.
    assert!(
        (28.5..36.0).contains(&gap),
        "중첩 표 윗변이 상자 윗변 아래 {gap:.1}pt — 맥 35.0pt(빈 문단 25.2pt 아래)여야 한다"
    );
}

#[test]
fn a_tall_nested_row_splits_by_lines_at_the_page_bottom() {
    let doc = doc();
    assert_eq!(doc.page_count(), 3, "쪽 수 — 맥 한글 12.30 도 3쪽");

    // 1쪽: 중첩 표가 쪽 바닥(맥 760.3pt)까지 차 있다. 행 5 를 통째 넘기면 540.6pt 에서 끝난다.
    let (_, nested1) = box_and_nested(&doc, 0);
    let nested1 = nested1.expect("1쪽 중첩 표");
    let bottom1 = pt(nested1.bbox.y + nested1.bbox.height);
    assert!(
        bottom1 > 740.0,
        "1쪽 중첩 표가 {bottom1:.1}pt 에서 끝난다 — 행 5 가 쪽 바닥까지 줄 단위로 채워야 한다(맥 760.3pt)"
    );

    // 2쪽: 행 5 의 나머지로 시작하고 쪽을 채운다(맥 (77.9, 178.6) 로 시작 · 649.8+109.1 까지).
    let (_, nested2) = box_and_nested(&doc, 1);
    let nested2 = nested2.expect("2쪽 중첩 표");
    let top2 = pt(nested2.bbox.y);
    let bottom2 = pt(nested2.bbox.y + nested2.bbox.height);
    assert!(
        top2 < 85.0,
        "2쪽 중첩 표가 {top2:.1}pt 에서 시작한다 — 쪽 머리(맥 73.8pt 상자 안)여야 한다"
    );
    assert!(
        bottom2 > 740.0,
        "2쪽 중첩 표가 {bottom2:.1}pt 에서 끝난다 — 쪽 바닥까지 차야 한다(맥 758.9pt)"
    );

    // 3쪽: 나머지 행 8 조각 + 서명 문단.
    let (_, nested3) = box_and_nested(&doc, 2);
    assert!(nested3.is_some(), "3쪽에 중첩 표 마지막 조각");
}
