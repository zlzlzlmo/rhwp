//! 오프셋 0 자리차지 표의 제어 문자가 글자처럼 표 줄 **뒤** 줄에 실리면, 한/글은 글자처럼 표를 먼저 두고 자리차지
//! 표를 그 줄 끝 + 줄 간격 자리에 앉힌다(맥 한글 12.30).
//!
//! ## 형상
//!
//! `samples/pic-in-head-01.hwp` 문단 32 = [글자처럼 표 ci0(줄0) · 자리차지 표 ci1(vert=문단 0) · 글자처럼 표 ci2(줄1)].
//! 줄0 저장 3648 · lh 20232 · ls 720, 줄1 은 쪽 초기화(저장 0). ci1 의 제어 문자는 줄1(text_start 9)에 있다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! 10쪽: ci0 표 글줄 112.0pt(줄0 자리) · ci1 첫 글줄 321.5pt = 줄0 끝 + 간격 + 바깥 위 여백(24741HU). 종전 rhwp 는 ci1
//! 을 문단 위에 먼저 두고 ci0 를 그 아래로 밀었다(글줄 583.5pt).

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/pic-in-head-01.hwp";

fn find_table<'a>(node: &'a RenderNode, pi: usize, ci: usize) -> Option<&'a RenderNode> {
    if matches!(&node.node_type,
        RenderNodeType::Table(t) if t.para_index == Some(pi) && t.control_index == Some(ci))
    {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_table(c, pi, ci))
}

#[test]
fn a_zero_offset_float_anchored_after_a_tac_line_sits_below_it() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    assert_eq!(doc.page_count(), 22, "정답지·맥 22쪽");
    let root = doc.build_page_render_tree(9).expect("10쪽").root;
    let body_top = 83.1; // 본문 위(px)
    let tac = find_table(&root, 32, 0).expect("문단 32 글자처럼 표");
    let float = find_table(&root, 32, 1).expect("문단 32 자리차지 표");
    assert!(
        tac.bbox.y < float.bbox.y,
        "글자처럼 표({:.1})가 자리차지 표({:.1})보다 위여야 한다",
        tac.bbox.y,
        float.bbox.y
    );
    // 자리차지 표 윗변 = 본문 위 + (24741 − 3648 + 3648)HU — 저장 좌표가 본문 위에서 시작한다.
    let expected = body_top + 24741.0 * 96.0 / 7200.0;
    assert!(
        (float.bbox.y - expected).abs() <= 1.0,
        "자리차지 표 윗변 {:.1} 이 맥 자리 {expected:.1} 이 아니다",
        float.bbox.y
    );
}
