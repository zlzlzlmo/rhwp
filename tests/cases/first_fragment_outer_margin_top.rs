//! 쪽 중간에서 시작하는 나눔 표의 첫 조각도 바깥 위 여백을 연다(A단계 ①) — 조판은 이미 그만큼 예약해 두었고(host 앞
//! 간격 = 바깥 위 여백), 그림만 빠뜨려 표가 여백만큼 위에 떴다. 행 중간에서 가른 비끝 조각의 상자는 본문 아래 − 바깥
//! 아래 여백 − 100HU 에서 끝난다(1×1 쪽 조각 #7095 · 빈 띠 가름과 같은 선) — 가른 행 높이에 든 마지막 줄 뒤 간격과 아래
//! 여백을 한/글은 그 선에서 자른다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! `samples/issue5885/3171199_design_capability_criteria.hwp` 1쪽 8×4 표(바깥 여백 283HU): 표 위 괘선 142.1pt(rhwp 종전
//! 139.3) · 첫 조각 아래 753.0pt = 본문 아래 756.9 − 2.83 − 1.00(rhwp 종전 755.7, 여백을 열면 758.5 로 본문을 넘었다).
//! 같은 규칙으로 맥 대조 말뭉치 80168 17쪽 · 76076 10쪽이 맥과 같아졌다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/issue5885/3171199_design_capability_criteria.hwp";
const PT_TO_PX: f64 = 96.0 / 72.0;

fn find<'a>(node: &'a RenderNode, pred: &dyn Fn(&RenderNode) -> bool) -> Option<&'a RenderNode> {
    if pred(node) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, pred))
}

#[test]
fn a_mid_page_first_fragment_opens_its_outer_top_margin_and_ends_at_the_cut_line() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    let root = doc.build_page_render_tree(0).expect("1쪽").root;

    let table = find(
        &root,
        &|node| matches!(&node.node_type, RenderNodeType::Table(t) if t.para_index == Some(3)),
    )
    .expect("문단 3 표");
    let body = find(&root, &|node| {
        matches!(node.node_type, RenderNodeType::Body { .. })
    })
    .expect("본문");

    let top = table.bbox.y;
    assert!(
        (top - 142.1 * PT_TO_PX).abs() <= 0.5,
        "첫 조각 위 {top:.2}px 가 맥 142.1pt({:.2}px)가 아니다 — 바깥 위 여백 283HU 를 열어야 한다",
        142.1 * PT_TO_PX
    );

    let bottom = table.bbox.y + table.bbox.height;
    let cut_line = body.bbox.y + body.bbox.height - (283.0 + 100.0) * 96.0 / 7200.0;
    assert!(
        (bottom - cut_line).abs() <= 0.5,
        "첫 조각 아래 {bottom:.2}px 가 자르는 선 {cut_line:.2}px(맥 753.0pt)가 아니다"
    );
}
