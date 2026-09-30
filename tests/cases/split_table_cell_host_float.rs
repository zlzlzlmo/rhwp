//! 글자처럼 형제 **뒤**에 달린 본문 자리차지 표의 칸 문단에 단 «쪽 영역 안으로 제한» 끈 글앞 그림(문단 기준)은
//! 표가 끝난 쪽, 표 아래 본문 문단 줄(표 아래 + 바깥 아래 여백 · 가로 단 왼쪽 + 문단 여백)을 원점으로 선다.
//!
//! 맥 한글 12.30 실측(위비즈 209차 — 서초 [붙임2]: 문단 1 = 글자처럼 제목 표 + 쪽 테두리 1×1 표, 오프셋 0 그림):
//! 세 쪽 표면 3쪽 칸 문단에 달든 1쪽 첫 칸 문단에 달든 3쪽 (75.5, 502.4)px — 표 아래 498.7 + 바깥 아래 여백 3.8 ·
//! 단 왼쪽 75.6. 한 쪽 표면 1003.2 = 999.4 + 3.8. 제목 표를 지운 같은 문서 · 형제 없는 두 쪽 14×9 표(b7ef0592)는
//! 종전대로 첫 쪽 문단 위다. 고치기 전 rhwp 는 나눈 표의 칸 그림을 어느 쪽에도 그리지 않았다.
//! 실제 사용자 문서의 문자열·그림을 담지 않는 공개 합성 IR 이다.
use rhwp::model::control::Control;
use rhwp::model::document::{Document, Section, SectionDef};
use rhwp::model::image::Picture;
use rhwp::model::page::PageDef;
use rhwp::model::paragraph::{LineSeg, Paragraph};
use rhwp::model::shape::{CommonObjAttr, HorzRelTo, TextWrap, VertRelTo};
use rhwp::model::style::ParaShape;
use rhwp::model::table::{Cell, Table, TablePageBreak};
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};
use rhwp::DocumentCore;

const HU_PER_PX: f64 = 75.0;
const CELL_PARAS: usize = 80;
const OUTER_BOTTOM_HU: i16 = 283;

fn line(y: i32) -> LineSeg {
    LineSeg {
        text_start: 0,
        vertical_pos: y,
        line_height: 800,
        text_height: 800,
        baseline_distance: 640,
        line_spacing: 200,
        segment_width: 30_000,
        tag: 393216,
        ..Default::default()
    }
}

fn host_float(horizontal: i32, vertical: i32) -> Control {
    Control::Picture(Box::new(Picture {
        common: CommonObjAttr {
            width: 3_000,
            height: 1_500,
            treat_as_char: false,
            flow_with_text: false,
            text_wrap: TextWrap::InFrontOfText,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Para,
            horizontal_offset: horizontal as u32,
            vertical_offset: vertical as u32,
            ..Default::default()
        },
        ..Default::default()
    }))
}

/// 본문 문단 하나에 (글자처럼 1×1 제목 표 +) 1×1 자리차지 표 — 칸 문단 80줄이면 한 쪽 본문보다 길다.
/// 그림은 첫 칸 문단과 끝 무렵 칸 문단에.
fn fixture(cell_paras: usize, tac_sibling: bool) -> Document {
    let paragraphs: Vec<Paragraph> = (0..cell_paras)
        .map(|i| {
            let mut para = Paragraph {
                char_count: 1,
                line_segs: vec![line(i as i32 * 1_000)],
                ..Default::default()
            };
            if i == 0 || i == cell_paras - 3 {
                para.controls.push(if i == 0 {
                    host_float(0, 0)
                } else {
                    host_float(1_500, -750)
                });
                para.char_count = 9;
            }
            para
        })
        .collect();
    let mut table = Table {
        row_count: 1,
        col_count: 1,
        page_break: TablePageBreak::RowBreak,
        outer_margin_bottom: OUTER_BOTTOM_HU,
        common: CommonObjAttr {
            width: 36_000,
            height: (cell_paras as u32) * 1_000,
            treat_as_char: false,
            flow_with_text: true,
            text_wrap: TextWrap::TopAndBottom,
            vert_rel_to: VertRelTo::Para,
            horz_rel_to: HorzRelTo::Column,
            ..Default::default()
        },
        cells: vec![Cell {
            row_span: 1,
            col_span: 1,
            width: 36_000,
            height: (cell_paras as u32) * 1_000,
            paragraphs,
            ..Default::default()
        }],
        ..Default::default()
    };
    table.rebuild_grid();
    let mut controls = vec![Control::Table(Box::new(table))];
    if tac_sibling {
        let mut title = Table {
            row_count: 1,
            col_count: 1,
            common: CommonObjAttr {
                width: 36_000,
                height: 2_000,
                treat_as_char: true,
                text_wrap: TextWrap::TopAndBottom,
                ..Default::default()
            },
            cells: vec![Cell {
                row_span: 1,
                col_span: 1,
                width: 36_000,
                height: 2_000,
                paragraphs: vec![Paragraph::default()],
                ..Default::default()
            }],
            ..Default::default()
        };
        title.rebuild_grid();
        controls.insert(0, Control::Table(Box::new(title)));
    }
    let host = Paragraph {
        char_count: 1 + 8 * controls.len() as u32,
        controls,
        line_segs: vec![line(0)],
        ..Default::default()
    };
    let mut doc = Document::default();
    doc.doc_info.para_shapes = vec![ParaShape::default()];
    doc.sections.push(Section {
        paragraphs: vec![host],
        section_def: SectionDef {
            page_def: PageDef {
                width: 44_000,
                height: 60_000,
                margin_left: 2_000,
                margin_right: 2_000,
                margin_top: 2_000,
                margin_bottom: 2_000,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
    doc
}

#[derive(Debug, Clone, Copy)]
struct Box2 {
    x: f64,
    y: f64,
    h: f64,
}

/// 쪽의 칸 그림(그림 또는 그림 없음 자리) 사각형 — 칸 문맥이 있는 것만.
fn cell_pictures(node: &RenderNode, out: &mut Vec<Box2>) {
    let in_cell = match &node.node_type {
        RenderNodeType::Image(image) => image.cell_context.is_some(),
        RenderNodeType::Placeholder(_) => true,
        _ => false,
    };
    if in_cell {
        out.push(Box2 {
            x: node.bbox.x,
            y: node.bbox.y,
            h: node.bbox.height,
        });
    }
    for child in &node.children {
        cell_pictures(child, out);
    }
}

/// 본문 표(문단 0 의 컨트롤 `control`) 조각의 아래 끝.
fn table_bottom(node: &RenderNode, control: usize) -> Option<f64> {
    if let RenderNodeType::Table(table) = &node.node_type {
        if table.cell_context.is_none() && table.control_index == Some(control) {
            return Some(node.bbox.y + node.bbox.height);
        }
    }
    node.children
        .iter()
        .find_map(|child| table_bottom(child, control))
}

fn core(cell_paras: usize, tac_sibling: bool) -> DocumentCore {
    let mut core = DocumentCore::new_empty();
    core.set_document(fixture(cell_paras, tac_sibling));
    core
}

fn pictures_on(core: &DocumentCore, page: u32) -> Vec<Box2> {
    let mut pictures = Vec::new();
    cell_pictures(
        &core.build_page_render_tree(page).unwrap().root,
        &mut pictures,
    );
    pictures.sort_by(|a, b| a.x.total_cmp(&b.x));
    pictures
}

/// 원점 + 오프셋: 첫 칸 문단 그림(0, 0) · 끝 무렵 칸 문단 그림(가로 +20px · 세로 −10px).
fn assert_host_line(pictures: &[Box2], host_y: f64) {
    let column_x = 2_000.0 / HU_PER_PX;
    assert_eq!(pictures.len(), 2, "칸 그림: {pictures:?}");
    assert!((pictures[0].x - column_x).abs() < 0.5, "{pictures:?}");
    assert!(
        (pictures[0].y - host_y).abs() < 0.5,
        "{pictures:?} host_y={host_y}"
    );
    assert!((pictures[0].h - 20.0).abs() < 0.5);
    assert!(
        (pictures[1].x - (column_x + 20.0)).abs() < 0.5,
        "{pictures:?}"
    );
    assert!(
        (pictures[1].y - (host_y - 10.0)).abs() < 0.5,
        "{pictures:?}"
    );
}

#[test]
fn split_table_after_tac_sibling_floats_stand_on_the_host_line_after_the_last_fragment() {
    let core = core(CELL_PARAS, true);
    assert_eq!(core.page_count(), 2, "{}", core.dump_page_items(None));
    let first = pictures_on(&core, 0);
    assert!(first.is_empty(), "첫 쪽에 선 칸 그림: {first:?}");
    let last = core.build_page_render_tree(1).unwrap();
    let bottom = table_bottom(&last.root, 1).expect("끝 쪽 표 조각");
    assert_host_line(
        &pictures_on(&core, 1),
        bottom + f64::from(OUTER_BOTTOM_HU) / HU_PER_PX,
    );
}

#[test]
fn unsplit_table_after_tac_sibling_floats_stand_below_the_table() {
    let core = core(20, true);
    assert_eq!(core.page_count(), 1, "{}", core.dump_page_items(None));
    let page = core.build_page_render_tree(0).unwrap();
    let bottom = table_bottom(&page.root, 1).expect("표");
    assert_host_line(
        &pictures_on(&core, 0),
        bottom + f64::from(OUTER_BOTTOM_HU) / HU_PER_PX,
    );
}

#[test]
fn split_table_without_tac_sibling_keeps_the_old_origin() {
    // 형제 없는 표는 한/글이 첫 쪽 문단 위에서 잰다(b7ef0592) — 끝 쪽 표 아래로 옮기지 않는다.
    let core = core(CELL_PARAS, false);
    assert_eq!(core.page_count(), 2, "{}", core.dump_page_items(None));
    let last = core.build_page_render_tree(1).unwrap();
    let host_y = table_bottom(&last.root, 0).expect("끝 쪽 표 조각")
        + f64::from(OUTER_BOTTOM_HU) / HU_PER_PX;
    let moved: Vec<_> = pictures_on(&core, 1)
        .into_iter()
        .filter(|p| (p.y - host_y).abs() < 11.0)
        .collect();
    assert!(moved.is_empty(), "끝 쪽 표 아래로 옮긴 칸 그림: {moved:?}");
}
