//! 저장 사다리가 줄 간격 전부를 증언하는 외톨이 글자처럼 표 줄은 조판도 줄 간격 전부만큼 흐른다 — 그래야 조판 자리가
//! 그림(=저장 사다리·한/글) 자리와 같아진다. 조판이 정확해지면 그동안 «조판이 몇 px 위» 덕에 가려져 있던 쪽 끝 규칙
//! 넷이 드러나서 함께 맞췄다. 모두 맥 한글 12.30 PDF 실측이다.
//!
//! 1. 빈 host 자리차지 표 뒤 문단의 조판 자리(규칙 M) — hwpctl 101쪽 pi 2706 이 저장 되감김에서 갈린다.
//! 2. 선언 빈 띠 안의 짧은 꼬리(12.8pt 미만)는 버리고 행을 자르는 선(본문 아래 − 바깥 아래 여백)에서 끝낸다 —
//!    rowbreak-problem-pages 3쪽 25×7 표 행 8(선언 67.08pt → 맥 64.4pt).
//! 3. 첫 조각 프레임이 행 안에서 끝나고 그 행 셀에 저장 vpos 되감김이 있으면 행 끝까지 넘치게 두지 않는다 —
//!    1480000 화학 표시 7쪽 pi 70 (맥: 행 2 를 11·11·8 줄에서 가른다).
//! 4. 첫 줄이 문단 앞 간격(0..=500HU)만큼 내려앉은 셀은 그 자리로의 되감김도 쪽 경계다 — `issue_1749` 의
//!    `issue_1811_hwpx_pi52_rowbreak_cut_matches_hwp_reference` 가 HWP 세 유닛 컷을 지킨다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

fn load(sample: &str) -> HwpDocument {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {sample}: {e}"));
    HwpDocument::from_bytes(&bytes).unwrap_or_else(|e| panic!("parse {sample}: {e:?}"))
}

fn item_line<'a>(page: &'a str, needle: &str) -> &'a str {
    page.lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} 없음\n{page}"))
}

/// hwpctl 101쪽 끝 pi 2706 은 저장 사다리가 둘째 줄에서 쪽을 되감는다(63516 → 0). 맥은 첫 줄만 101쪽에 두고 둘째
/// 줄로 102쪽을 연다 — 종전 조판은 외톨이 글자처럼 표 줄의 줄 간격 절반만 쌓아 두 줄을 다 101쪽에 얹었다(+13.2pt).
#[test]
fn hwpctl_page_101_splits_at_the_stored_rewind() {
    let doc = load("samples/hwpctl_API_v2.4.hwp");
    let p101 = doc.dump_page_items(Some(100));
    let p102 = doc.dump_page_items(Some(101));
    assert!(
        item_line(&p101, "pi=2706 ").contains("PartialParagraph  pi=2706  lines=0..1"),
        "101쪽은 pi 2706 첫 줄로 끝나야 한다\n{p101}"
    );
    assert!(
        item_line(&p102, "pi=2706 ").contains("PartialParagraph  pi=2706  lines=1..2"),
        "102쪽은 pi 2706 둘째 줄로 열려야 한다\n{p102}"
    );
}

fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    out.push(node);
    for child in &node.children {
        collect(child, out);
    }
}

/// rowbreak-problem-pages 3쪽 25×7 표 행 8 은 칸 선언 높이(67.08pt)가 글(4줄 + 여백 49.8pt)보다 크다. 맥은 행 8 을
/// 3쪽에 두고 선언 띠의 꼬리 2.7pt 를 버려 표를 본문 아래 − 바깥 아래 여백(141HU)에서 끝낸다. 4쪽은 행 9 부터다.
#[test]
fn rowbreak_row_with_a_short_blank_band_tail_ends_at_the_cut_line() {
    let doc = load("samples/rowbreak-problem-pages.hwp");
    let p3 = doc.dump_page_items(Some(2));
    let p4 = doc.dump_page_items(Some(3));
    assert!(
        item_line(&p3, "pi=11 ").contains("rows=0..9  cont=false"),
        "3쪽 첫 조각은 행 8 까지 담아야 한다\n{p3}"
    );
    assert!(
        item_line(&p4, "pi=11 ").contains("rows=9..16  cont=true"),
        "4쪽 조각은 행 9 부터여야 한다\n{p4}"
    );

    let root = doc.build_page_render_tree(2).expect("3쪽").root;
    let mut nodes = Vec::new();
    collect(&root, &mut nodes);
    let table_bottom = nodes
        .iter()
        .find_map(|node| match &node.node_type {
            RenderNodeType::Table(table) if table.para_index == Some(11) => {
                Some(node.bbox.y + node.bbox.height)
            }
            _ => None,
        })
        .expect("pi 11 표");
    // 본문 아래 = 용지 297mm − 아래 여백 15mm − 꼬리말 10mm = 272mm, 자르는 선은 그보다 바깥 아래 여백 141HU 위.
    let body_bottom = 272.0 / 25.4 * 96.0;
    let cut_line = body_bottom - 141.0 * 96.0 / 7200.0;
    assert!(
        table_bottom <= cut_line + 0.5,
        "3쪽 표 바닥 {table_bottom:.2}px 가 자르는 선 {cut_line:.2}px 를 넘는다"
    );
}

/// 1480000 화학 표시 7쪽 pi 70 (3×4 나눔 표): 저장 첫 조각 프레임 256.4px 가 행 2(측정 300px) 안에서 끝나고 행 2
/// 셀 줄이 11번째 줄 뒤 vpos 0 으로 되감긴다. 맥은 행 2 를 11·11·8 줄에서 가른다 — 되감김 분기가 행 끝까지 96.8px 를
/// 허용해 표를 통째 받으면 본문을 넘긴다.
#[test]
fn a_first_fragment_frame_inside_a_rewinding_row_does_not_own_the_whole_row() {
    let doc = load("samples/issue6782/1480000-201900042-chemical-labeling-standards.hwp");
    let p7 = doc.dump_page_items(Some(6));
    let line = item_line(&p7, "pi=70 ");
    assert!(
        line.contains("PartialTable   pi=70 ci=0  rows=0..3")
            && line.contains("end_cut=[1, 11, 11, 8]"),
        "7쪽 pi 70 은 행 2 를 11·11·8 줄에서 갈라야 한다\n{line}"
    );
}
