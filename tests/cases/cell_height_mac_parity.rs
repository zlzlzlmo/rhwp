//! 칸 높이 셋 — 맥 한글 12.30 PDF 실측으로 세운 규칙.
//!
//! 1. 칸 끝 줄 간격은 칸 높이에 넣지 않는다(#1763). 칸 안에 중첩 표가 있어도 **마지막 문단이 평문**이면 같다.
//!    `samples/sign-forms/consent-checkboxes.hwp` 의 1×1 바깥 표(문단 41 · 중첩 표 둘)는 저장 내용 78396HU + 칸 여백
//!    282HU = 선언 78678HU(맥 괘선 33.2 → 820.0pt = 786.8pt). 끝 줄 간격 600HU 를 넣으면 행이 8px 커지고, 칸 세로
//!    가운데 정렬이 그 여유를 반으로 나눠 칸 속 전부가 3.1pt 아래로 밀렸다.
//! 2. 비정상 세로 여백(여백 합 ≥ 칸 선언) 칸의 내용이 선언을 넘으면 측정기는 원 여백으로 행을 키우고 레이아웃도 그
//!    높이로 그린다 — 조판 예약도 같은 높이여야 한다. `samples/issue6697/80550-…hwpx` 27쪽 pi208 1~3행은 맥 17.4pt
//!    (내용 13pt + 원 여백). 여백을 줄인 컷 높이(15.2pt)로 예약하면 한 행이 더 들어가 본문을 11.9px 넘는다.
//!    (한 행짜리 표의 선언 높이 규칙은 `issue_2308`·`issue_3128` 정답지 핀이 잰다.)

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const HU_PER_PX: f64 = 7200.0 / 96.0;

fn document(sample: &str) -> HwpDocument {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("fixture 를 읽을 수 없다 ({}): {error}", path.display()));
    HwpDocument::from_bytes(&bytes).expect("문서 로드")
}

fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    out.push(node);
    for child in &node.children {
        collect(child, out);
    }
}

fn table_node<'a>(nodes: &[&'a RenderNode], para_index: usize) -> &'a RenderNode {
    nodes
        .iter()
        .copied()
        .find(|node| {
            matches!(&node.node_type, RenderNodeType::Table(table) if table.para_index == Some(para_index))
        })
        .unwrap_or_else(|| panic!("문단 {para_index} 표가 없다"))
}

#[test]
fn a_nested_cell_excludes_its_last_line_spacing_from_the_row() {
    let doc = document("samples/sign-forms/consent-checkboxes.hwp");
    let root = doc.build_page_render_tree(0).expect("1쪽").root;
    let mut nodes = Vec::new();
    collect(&root, &mut nodes);
    let table = table_node(&nodes, 2);
    let declared = 78678.0 / HU_PER_PX;
    assert!(
        (table.bbox.height - declared).abs() <= 0.5,
        "바깥 표 높이 {:.2}px 가 선언 78678HU({declared:.2}px, 맥 786.8pt)와 다르다 — 칸 끝 줄 간격이 들어갔다",
        table.bbox.height
    );
}

#[test]
fn an_overflowing_abnormal_padding_row_reserves_its_painted_height() {
    let doc = document("samples/issue6697/80550-agricultural-machinery-act-amendment.hwpx");
    let root = doc.build_page_render_tree(26).expect("27쪽").root;
    let mut nodes = Vec::new();
    collect(&root, &mut nodes);
    let body_bottom = nodes
        .iter()
        .find(|node| matches!(node.node_type, RenderNodeType::Body { .. }))
        .map(|node| node.bbox.y + node.bbox.height)
        .expect("Body");
    let table = table_node(&nodes, 208);
    let mut inner = Vec::new();
    collect(table, &mut inner);
    let last_row = inner
        .iter()
        .filter_map(|node| match &node.node_type {
            RenderNodeType::TableCell(cell) => Some(cell.row),
            _ => None,
        })
        .max()
        .expect("칸");
    assert_eq!(
        last_row, 3,
        "맥 한글 12.30 은 27쪽에 pi208 네 행(0~3, 괘선 707.8 → 777.7pt)만 둔다"
    );
    assert!(
        table.bbox.y + table.bbox.height <= body_bottom + 0.5,
        "pi208 첫 조각 바닥 {:.2} 가 본문 바닥 {body_bottom:.2} 를 넘는다",
        table.bbox.y + table.bbox.height
    );
}
