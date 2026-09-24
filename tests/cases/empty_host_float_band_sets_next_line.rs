//! 빈 host 문단 기준 자리차지 표 뒤 글 문단의 첫 줄은 띠 바닥(표 아래 + 바깥 아래 여백)에 선다(맥 한글 12.30).
//!
//! ## 형상
//!
//! `samples/hwpctl_API_v2.4.hwp` 16쪽 문단 282 는 빈 host 에 달린 1×1 자리차지 표(선언 2882 HU · 바깥 여백 283 HU ·
//! host 앞/뒤 간격 500 HU)이고, 문단 283 은 앞 간격 500 HU 짜리 빈 문단이다.
//!
//! ```text
//!   보통 흐름   host 줄 14457 + 1000 + 300 + host 뒤 500 + 다음 앞 500 = 16757
//!   띠 바닥     host 문단 위 13957 + 위 여백 283 + 2882 + 아래 여백 283  = 17405   ← 저장 첫 줄
//! ```
//!
//! 한/글은 띠를 가로지르는 줄만 띠 아래로 내리고 두 간격은 띠 안에 흡수한다. rhwp 레이아웃은 띠 뒤에 host 뒤 간격과
//! 다음 앞 간격을 더 얹어 문단 283 이하가 10pt 아래였고, 되감기 8px 제한이 저장 자리로의 복귀를 막았다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! 16쪽 가로선 353.5 · 532.3 · 551.0 · 718.0pt(rhwp 종전 363.5 · 542.3 · 561.0 · 728.0) — 저장 사다리 17405 와 같다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/hwpctl_API_v2.4.hwp";

fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    out.push(node);
    for child in &node.children {
        collect(child, out);
    }
}

#[test]
fn the_line_after_an_empty_host_float_band_sits_at_the_band_bottom() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    let root = doc.build_page_render_tree(15).expect("16쪽").root;
    let mut nodes = Vec::new();
    collect(&root, &mut nodes);

    let table_bottom = nodes
        .iter()
        .find_map(|node| match &node.node_type {
            RenderNodeType::Table(table) if table.para_index == Some(282) => {
                Some(node.bbox.y + node.bbox.height)
            }
            _ => None,
        })
        .expect("문단 282 표");
    let next_line_top = nodes
        .iter()
        .find_map(|node| match &node.node_type {
            RenderNodeType::TextLine(line) if line.para_index == Some(283) => Some(node.bbox.y),
            _ => None,
        })
        .expect("문단 283 줄");

    let outer_bottom_px = 283.0 * 96.0 / 7200.0;
    let gap = next_line_top - table_bottom;
    assert!(
        (gap - outer_bottom_px).abs() <= 0.5,
        "표 바닥 {table_bottom:.2} → 문단 283 첫 줄 {next_line_top:.2}: 간격 {gap:.2}px 가 바깥 아래 여백 \
         {outer_bottom_px:.2}px 가 아니다(host 뒤·다음 앞 간격을 얹으면 {:.2})",
        outer_bottom_px + 1000.0 * 96.0 / 7200.0
    );
}
