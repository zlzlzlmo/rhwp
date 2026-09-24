//! 빈 host 1×1 나눔 표 뒤 흐름에는 바깥 아래 여백이 한 번만 든다(맥 한글 12.30).
//!
//! ## 형상
//!
//! `samples/80168_regulatory_analysis.hwp`(저장 줄이 없는 문서) 29쪽의 문단 223 은 빈 host 에 달린 1×1 나눔 자리차지
//! 표(바깥 여백 141 HU)다. 두 흐름 계약이 겹쳤다.
//!
//! ```text
//!   empty_rowbreak_flow_end   그린 바닥 + 바깥 아래 여백            (76076 p33 계약)
//!   rhwp_composed_host        흐름 + 바깥 아래 여백                 (줄 없는 host 계약)
//! ```
//!
//! 앞 계약이 이미 여백을 흘렸는데 뒤 계약이 또 더해 표 뒤 빈 문단(224)이 2.8pt 아래에 섰다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! 표 괘선 317.40 → 476.16pt · 다음 제목(문단 225) 기준선 511.9pt = 표 바닥 + 1.41(여백) + 22.43(문단 224)
//! + 11.9(기준선). 즉 표 바닥과 문단 224 윗변 사이는 바깥 아래 여백 한 번이다.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/80168_regulatory_analysis.hwp";
/// 141 HU 를 px 로.
const OUTER_MARGIN_BOTTOM_PX: f64 = 141.0 * 96.0 / 7200.0;

fn collect<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderNode>) {
    out.push(node);
    for child in &node.children {
        collect(child, out);
    }
}

#[test]
fn an_empty_host_single_cell_rowbreak_table_opens_its_bottom_margin_once() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    let root = doc.build_page_render_tree(28).expect("29쪽").root;
    let mut nodes = Vec::new();
    collect(&root, &mut nodes);

    let table_bottom = nodes
        .iter()
        .find_map(|node| match &node.node_type {
            RenderNodeType::Table(table) if table.para_index == Some(223) => {
                Some(node.bbox.y + node.bbox.height)
            }
            _ => None,
        })
        .expect("문단 223 표");
    let next_line_top = nodes
        .iter()
        .find_map(|node| match &node.node_type {
            RenderNodeType::TextLine(line) if line.para_index == Some(224) => Some(node.bbox.y),
            _ => None,
        })
        .expect("문단 224 줄");

    let gap = next_line_top - table_bottom;
    assert!(
        (gap - OUTER_MARGIN_BOTTOM_PX).abs() <= 0.2,
        "표 바닥 {table_bottom:.2} → 문단 224 {next_line_top:.2}: 간격 {gap:.2}px 가 바깥 아래 여백 \
         {OUTER_MARGIN_BOTTOM_PX:.2}px 한 번이 아니다(두 번이면 {:.2})",
        2.0 * OUTER_MARGIN_BOTTOM_PX
    );
}
