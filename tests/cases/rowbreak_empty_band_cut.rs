//! 나눔(RowBreak) 표의 빈 띠 행은 쪽 끝에서 띠째 갈린다(맥 한글 12.30).
//!
//! ## 형상
//!
//! `samples/rowbreak-empty-band/gyeongbuk_sales_support_form.hwp` 2쪽의 6×1 표(문단 7)는 머리 행과 빈 칸
//! 행이 번갈아 선다. 빈 칸은 글 한 줄(빈 문단)에 선언 높이가 386.9pt 라 행 높이를 **글 아래 빈 띠**가
//! 정한다. 종전 rhwp 는 한 줄짜리 행을 «가를 수 없는 행»(그림 칸 가드)으로 보고 통째 넘겨 쪽이 하나
//! 늘었다(17쪽).
//!
//! ## 기대값의 출처 — 맥 한글 12.30 PDF 실측
//!
//! ```text
//!   쪽 수                  16
//!   2쪽 행 3 조각 아래     789.00pt   = 본문 아래 790.00pt − 1.00pt
//!   3쪽 이어진 띠          70.92 → 269.16pt (198.24pt = 행 386.88 − 앞 조각 188.64)
//! ```
//!
//! 자르는 자리의 1.00pt(100 HU)는 표 위치(−19.2·0·+24·+48pt)·칸 여백·쪽 아래 여백(15·15.1·20mm)을 바꾼
//! 사본 아홉 개에서 모두 같았다(±0.01pt).
//!
//! 칸에 조판부호가 있으면 띠가 비어 있지 않다 — 간장 보고서 그림 8(글 밖 그림이 든 2×1 표)은 행을
//! 가르지 않는다(맥 12쪽 정합).

#![cfg(not(target_arch = "wasm32"))]

use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::wasm_api::HwpDocument;

const SAMPLE: &str = "samples/rowbreak-empty-band/gyeongbuk_sales_support_form.hwp";
const LIVER: &str =
    "samples/정책연구용역사업 중간진도보고서(살아있는 간장 기증자의 의학적 선별기준 연구).hwp";
const PT_TO_PX: f64 = 96.0 / 72.0;

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

/// 문단 `para_index` 표의 `row` 행 칸들(이 쪽에 그려진 것).
fn row_cells(root: &RenderNode, para_index: usize, row: u16) -> Vec<(f64, f64)> {
    let mut nodes = Vec::new();
    collect(root, &mut nodes);
    let mut out = Vec::new();
    for table in nodes.iter().filter(|node| match &node.node_type {
        RenderNodeType::Table(table) => table.para_index == Some(para_index),
        _ => false,
    }) {
        let mut inner = Vec::new();
        collect(table, &mut inner);
        out.extend(inner.iter().filter_map(|node| match &node.node_type {
            RenderNodeType::TableCell(cell) if cell.row == row => {
                Some((node.bbox.y, node.bbox.y + node.bbox.height))
            }
            _ => None,
        }));
    }
    out
}

#[test]
fn an_empty_band_row_splits_at_the_page_bottom_like_hangul_mac() {
    let doc = document(SAMPLE);
    assert_eq!(
        doc.page_count(),
        16,
        "맥 한글 12.30 은 16쪽이다(행 통째 이월이면 17쪽)"
    );

    // 2쪽(idx 1): 행 3 의 첫 조각이 본문 아래 − 1pt 에서 끝난다.
    let page2 = doc.build_page_render_tree(1).expect("2쪽").root;
    let first = row_cells(&page2, 7, 3);
    assert_eq!(first.len(), 1, "행 3 의 첫 조각이 2쪽에 있어야 한다");
    let cut = first[0].1;
    assert!(
        (cut - 789.00 * PT_TO_PX).abs() <= 0.5,
        "행 3 조각 아래 {cut:.2}px 가 맥 789.00pt({:.2}px)와 다르다",
        789.00 * PT_TO_PX
    );

    // 3쪽(idx 2): 남은 띠가 쪽 첫머리에서 이어지고 다음 머리 행(4)이 그 아래에 선다.
    let page3 = doc.build_page_render_tree(2).expect("3쪽").root;
    let tail = row_cells(&page3, 7, 3);
    assert_eq!(tail.len(), 1, "행 3 의 남은 띠가 3쪽 첫머리에 있어야 한다");
    let header = row_cells(&page3, 7, 4);
    assert_eq!(header.len(), 1, "머리 행 4 는 3쪽에 있다");
    assert!(
        (header[0].0 - 269.16 * PT_TO_PX).abs() <= 0.5,
        "머리 행 4 윗변 {:.2}px 가 맥 269.16pt({:.2}px)와 다르다 — 남은 띠 높이가 어긋났다",
        header[0].0,
        269.16 * PT_TO_PX
    );
}

/// 글 밖 그림이 든 칸은 빈 띠가 아니다 — 행을 가르지 않고 통째 둔다.
#[test]
fn a_cell_holding_a_picture_is_not_an_empty_band() {
    let doc = document(LIVER);
    let pages_with_row0: Vec<u32> = (11..=12)
        .filter(|&page| {
            let root = doc.build_page_render_tree(page).expect("쪽").root;
            !row_cells(&root, 250, 0).is_empty()
        })
        .collect();
    assert_eq!(
        pages_with_row0,
        vec![11],
        "그림 8 의 그림 칸(문단 250 행 0)은 한 쪽에만 그려져야 한다"
    );
}
