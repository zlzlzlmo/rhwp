//! 한/글 문단 모양의 쪽 나눔 보호 세 가지 — 맥 한글 12.30 쓸기 실측(2026-09-23)을 그대로 옮긴 계약.
//!
//! 문서: 한 줄 «채움» 문단 F개 → 한 줄 제목 H → 7줄로 감기는 본문 B → 끝 줄. 첫 쪽은 채움 41줄이 찬다.
//! 한/글이 낸 PDF에서 H·B가 선 쪽과 B의 쪽별 줄 수를 읽었다(F = 34..42, 속성마다 9개).

use crate::document_core::DocumentCore;
use crate::renderer::pagination::PageItem;

const BODY_REPEAT: usize = 20;

fn build(fill: usize, h_props: &str, b_props: &str) -> DocumentCore {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().unwrap();
    let mut lines: Vec<String> = (1..=fill).map(|i| format!("채움{i:03}")).collect();
    lines.push("제목줄HHH".to_string());
    lines.push(format!(
        "본문BBB{}끝BBB",
        "가나다라마바사아자차카타파하".repeat(BODY_REPEAT)
    ));
    lines.push("끝줄1".to_string());
    for (i, text) in lines.iter().enumerate() {
        core.insert_text_native(0, i, 0, text).unwrap();
        if i + 1 < lines.len() {
            core.split_paragraph_native(0, i, text.chars().count(), None)
                .unwrap();
        }
    }
    if !h_props.is_empty() {
        core.apply_para_format_native(0, fill, h_props).unwrap();
    }
    if !b_props.is_empty() {
        core.apply_para_format_native(0, fill + 1, b_props).unwrap();
    }
    core.reflow_linesegs_on_demand();
    core.paginate();
    core
}

/// 문단이 선 쪽마다 (쪽, 줄 수). 통째로 선 문단은 줄 수 대신 `usize::MAX`.
fn placement(core: &DocumentCore, para: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (page_idx, page) in core.pagination[0].pages.iter().enumerate() {
        for col in &page.column_contents {
            for item in &col.items {
                match item {
                    PageItem::FullParagraph { para_index } if *para_index == para => {
                        out.push((page_idx, usize::MAX))
                    }
                    PageItem::PartialParagraph {
                        para_index,
                        start_line,
                        end_line,
                    } if *para_index == para => out.push((page_idx, end_line - start_line)),
                    _ => {}
                }
            }
        }
    }
    out
}

fn page_of(core: &DocumentCore, para: usize) -> usize {
    placement(core, para)
        .first()
        .map(|(p, _)| *p)
        .expect("문단이 어느 쪽에도 없다")
}

#[test]
fn baseline_lines_of_the_sweep_match_hangul() {
    // 보호 없음: F=34 → B 6/1, F=39 → 1/6, F=40 → 제목만 첫 쪽 끝에 홀로 남는다(한/글도 같다).
    let core = build(34, "", "");
    assert_eq!(placement(&core, 35), vec![(0, 6), (1, 1)]);
    let core = build(39, "", "");
    assert_eq!(placement(&core, 40), vec![(0, 1), (1, 6)]);
    let core = build(40, "", "");
    assert_eq!((page_of(&core, 40), page_of(&core, 41)), (0, 1));
}

#[test]
fn keep_with_next_moves_heading_only_when_next_first_line_misses_the_page() {
    // F=39: B 첫 줄이 첫 쪽에 들어간다 → 제목은 그대로(B는 1/6으로 갈린다).
    let core = build(39, r#"{"keepWithNext":true}"#, "");
    assert_eq!(page_of(&core, 39), 0);
    assert_eq!(placement(&core, 40), vec![(0, 1), (1, 6)]);
    // F=40: B 첫 줄이 못 들어간다 → 제목이 B와 함께 다음 쪽.
    let core = build(40, r#"{"keepWithNext":true}"#, "");
    assert_eq!((page_of(&core, 40), page_of(&core, 41)), (1, 1));
}

#[test]
fn widow_orphan_keeps_two_lines_on_each_side() {
    // 끝 줄 하나만 넘어가던 자리(6/1) → 5/2.
    let core = build(34, "", r#"{"widowOrphan":true}"#);
    assert_eq!(placement(&core, 35), vec![(0, 5), (1, 2)]);
    // 첫 줄 하나만 남던 자리(1/6) → 문단째 다음 쪽.
    let core = build(39, "", r#"{"widowOrphan":true}"#);
    assert_eq!(placement(&core, 40), vec![(1, usize::MAX)]);
    // 사이(4/3)는 보호가 없을 때와 같다.
    let core = build(36, "", r#"{"widowOrphan":true}"#);
    assert_eq!(placement(&core, 37), vec![(0, 4), (1, 3)]);
}

#[test]
fn keep_lines_moves_a_paragraph_that_would_split() {
    for fill in [34, 36, 39] {
        let core = build(fill, "", r#"{"keepLines":true}"#);
        assert_eq!(
            placement(&core, fill + 1),
            vec![(1, usize::MAX)],
            "F={fill}"
        );
    }
}
