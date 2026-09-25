//! 빈 host 자리차지 표의 이어진 끝 조각 뒤 문단은 조각 바닥 + 바깥 아래 여백에 선다 — 한/글 저장 첫 줄이 증언하면
//! 레이아웃도 **아래로** 옮긴다(조판은 규칙 M 스냅으로 이미 같은 자리). 종전 레이아웃은 위로만 당겨, 여백이 빠진 자리에
//! 둔 뒤 쪽 전체를 그 기준으로 283HU 위에 그렸다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! `samples/hwpctl_API_v2.4.hwp` 13쪽은 1×1 표(문단 176)의 끝 조각으로 시작하고, 빈 문단 177 의 저장 첫 줄 3448HU =
//! 조각 바닥 + 283HU 다. 맥은 그 뒤 괘선을 저장 자리에 그린다(rhwp 종전 −2.8pt · 12·23·91쪽도 같은 형상).

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/hwpctl_API_v2.4.hwp";

fn find<'a>(node: &'a RenderNode, pred: &dyn Fn(&RenderNode) -> bool) -> Option<&'a RenderNode> {
    if pred(node) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, pred))
}

#[test]
fn the_paragraph_after_a_terminal_continuation_moves_down_to_the_stored_line() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    let root = doc.build_page_render_tree(12).expect("13쪽").root;

    let body = find(&root, &|node| {
        matches!(node.node_type, RenderNodeType::Body { .. })
    })
    .expect("본문");
    let remarks = find(&root, &|node| {
        matches!(&node.node_type, RenderNodeType::TextLine(line) if line.para_index == Some(178))
    })
    .expect("문단 178 줄");

    // 문단 178 의 저장 첫 줄 4748HU(쪽 기준) — 조각 바닥 + 283HU 에 선 빈 문단 177(3448) 다음 줄.
    let stored = 4748.0 * 96.0 / 7200.0;
    let top = remarks.bbox.y - body.bbox.y;
    assert!(
        (top - stored).abs() <= 0.6,
        "문단 178 첫 줄 {top:.2}px 가 저장 자리 {stored:.2}px 가 아니다 — 끝 조각 뒤 바깥 아래 여백을 열어야 한다"
    );
}
