//! 양수 세로 오프셋 자리차지 표의 제어 문자가 글자처럼 표 **뒤**로 같은 저장 줄에 실리고 선언 자리(줄 위 + 오프셋)가
//! 그 줄 안이면, 한/글은 글자처럼 표를 먼저 두고 자리차지 표의 세로 기준을 그 줄 끝 + 줄 간격(다음 줄 자리)으로
//! 내린다(맥 한글 12.30).
//!
//! ## 형상 — 위비즈가 채운 서초AICT 데모데이 [붙임2] 제출본(`samples/webiz-fill/seocho_title_band_order.hwpx`)
//!
//! 문단 1 = [글자처럼 제목 띠 ci0(1×1 · 3980 · 바깥 여백 140) · 자리차지 테두리 표 ci1(1×1 · vert=문단 889 · 바깥 여백
//! 283 · 쪽을 넘는 선언 66339)], 저장 줄은 하나(vpos 2400 · lh 4260 · ls 900). 표본은 칸 안 글·표·그림을 걷은 축소본이다.
//! 종전 rhwp 는 보조 키(비 TAC 먼저)로 테두리 표를 문단 위에 먼저 두고 제목 띠를 그 꼬리 뒤(원본 3쪽 바닥)로 밀었다.
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측(걷기 전 원본 · 같은 문단 기하)
//!
//! 1쪽: 제목 띠 윗변 96.4pt = 본문 위 + 2400 + 140 · 테두리 표 윗변 158.3pt = 본문 위 + 2400 + 4260 + 900 + 889 + 283.

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/webiz-fill/seocho_title_band_order.hwpx";

fn find_table<'a>(node: &'a RenderNode, pi: usize, ci: usize) -> Option<&'a RenderNode> {
    if matches!(&node.node_type,
        RenderNodeType::Table(t) if t.para_index == Some(pi) && t.control_index == Some(ci))
    {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_table(c, pi, ci))
}

fn hu(v: f64) -> f64 {
    v * 96.0 / 7200.0
}

#[test]
fn a_positive_offset_float_after_a_tac_on_its_line_sits_below_that_line() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).expect("fixture");
    let doc = HwpDocument::from_bytes(&bytes).expect("문서 로드");
    let root = doc.build_page_render_tree(0).expect("1쪽").root;
    let body_top = hu(4252.0 + 2835.0); // 위 여백 + 머리말
    let band = find_table(&root, 1, 0).expect("1쪽에 제목 띠(문단 1 ci0)");
    let frame = find_table(&root, 1, 1).expect("1쪽에 테두리 표(문단 1 ci1)");
    let band_top = body_top + hu(2400.0 + 140.0);
    let frame_top = body_top + hu(2400.0 + 4260.0 + 900.0 + 889.0 + 283.0);
    assert!(
        (band.bbox.y - band_top).abs() <= 1.0,
        "제목 띠 윗변 {:.1} 이 맥 자리 {band_top:.1} 이 아니다",
        band.bbox.y
    );
    assert!(
        (frame.bbox.y - frame_top).abs() <= 1.0,
        "테두리 표 윗변 {:.1} 이 맥 자리 {frame_top:.1} 이 아니다",
        frame.bbox.y
    );
}
