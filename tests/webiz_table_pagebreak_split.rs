//! 쪽 경계에서 «나누지 않음»(0)·«셀 단위로 나눔»(1)·«나눔»(2) 값이 한글 정품처럼 그려진다(위비즈 252차 · rhwp #7288).
//!
//! 한글 정품 12.30.0(빌드 6523) 열기 실측 — 글자처럼 취급이 아닌(자리차지) 새 표를 모두의 창업 양식 앞에 끼운 문서의 총 쪽 수(기준 문서 6쪽):
//!   · 60행 표(행이 짧다):     나누지 않음 7 · 셀 단위로 나눔 8 · 나눔 8   → 나누지 않음은 한 쪽에 갇히고(쪽 밖이 안 보인다) 나머지는 행 사이에서 쪽을 잇는다
//!   · 한 칸 90줄(칸이 쪽보다 큼): 나누지 않음 7 · 셀 단위로 나눔 7 · 나눔 10 → 칸이 쪽보다 크면 나눔만 줄 단위로 잇고 나머지는 한 쪽에서 잘린다
//! 이 시험은 같은 모양을 빈 문서에서 만들어 «표가 몇 쪽에 걸쳐 그려지나»(PartialTable 조각)로 잰다.

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::wasm_api::HwpDocument;

const NONE: u8 = 0;
const CELL: u8 = 1;
const ROW: u8 = 2;

/// `rows`×1 자리차지 표(글자처럼 취급 아님)에 쪽 경계 값을 건 문서 바이트. 칸마다 글을 넣는다 — `text_per_cell`은 칸 하나의 글자 수.
fn doc(rows: u16, page_break: u8, text_per_cell: usize) -> Vec<u8> {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().expect("blank document");
    core.create_table_ex_native(0, 0, 0, rows, 1, false, None, None)
        .expect("table");
    let control_idx = core.document().sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .expect("표 컨트롤");
    core.set_table_properties_native(
        0,
        0,
        control_idx,
        &format!(r#"{{"pageBreak":{page_break},"treatAsChar":false}}"#),
    )
    .expect("쪽 경계 속성");
    for cell in 0..rows as usize {
        let text = "가나다라마바사아자차카타파하".repeat(text_per_cell / 14 + 1);
        core.insert_text_in_cell_native(0, 0, control_idx, cell, 0, 0, &text)
            .expect("칸 글");
    }
    core.export_hwp_native().expect("export hwp")
}

fn reopened(bytes: &[u8]) -> HwpDocument {
    HwpDocument::from_bytes(bytes).unwrap_or_else(|e| panic!("parse: {e:?}"))
}

/// 표가 걸친 쪽들의 덤프를 이어 붙인 것 — 첫 쪽에서 쪽 수만큼.
fn dumps(bytes: &[u8]) -> (u32, String) {
    let d = reopened(bytes);
    let pages = d.page_count();
    let all = (0..pages)
        .map(|p| d.dump_page_items(Some(p)))
        .collect::<Vec<_>>()
        .join("\n---\n");
    (pages, all)
}

/// 표가 걸친 쪽 수와 그 쪽들의 PartialTable 조각 수.
fn shape(rows: u16, page_break: u8, text: usize) -> (u32, usize) {
    let (pages, dump) = dumps(&doc(rows, page_break, text));
    (pages, dump.matches("PartialTable").count())
}

// ── 60행 표(행이 짧다) — 한글: 나누지 않음은 한 쪽에 갇히고, 셀 단위·나눔은 행 사이에서 쪽을 잇는다 ──

#[test]
fn rows60_none_stays_on_one_page_and_is_not_split() {
    let (pages, partials) = shape(60, NONE, 12);
    assert_eq!(
        (pages, partials),
        (1, 0),
        "나누지 않음: 표를 가르지 않는다(쪽 밖은 잘린다)"
    );
}

#[test]
fn rows60_cell_unit_continues_between_rows() {
    let (pages, partials) = shape(60, CELL, 12);
    assert_eq!(pages, 2, "셀 단위로 나눔: 행 사이에서 다음 쪽으로 잇는다");
    assert!(partials >= 2, "PartialTable 조각 둘: {partials}");
}

#[test]
fn rows60_row_break_continues_between_rows() {
    let (pages, partials) = shape(60, ROW, 12);
    assert_eq!(pages, 2, "나눔: 쪽 경계에서 잇는다");
    assert!(partials >= 2, "PartialTable 조각 둘: {partials}");
}

// ── 한 칸이 쪽보다 큰 표 — 한글: 나눔만 줄 단위로 잇고, 나누지 않음·셀 단위는 한 쪽에서 잘린다 ──

#[test]
fn tall_cell_none_is_clipped_on_one_page() {
    let (pages, partials) = shape(1, NONE, 9000);
    assert_eq!(
        (pages, partials),
        (1, 0),
        "나누지 않음: 칸이 쪽보다 커도 이어 그리지 않는다"
    );
}

#[test]
fn tall_cell_cell_unit_is_clipped_on_one_page() {
    let (pages, partials) = shape(1, CELL, 9000);
    assert_eq!(
        (pages, partials),
        (1, 0),
        "셀 단위로 나눔: 셀 안은 나누지 않는다 — 쪽보다 큰 칸은 한 쪽에서 잘린다"
    );
}

#[test]
fn tall_cell_row_break_continues_by_lines() {
    let (pages, partials) = shape(1, ROW, 9000);
    assert!(
        pages >= 3,
        "나눔: 칸 안 줄을 다음 쪽으로 잇는다 — 쪽 수 {pages}"
    );
    assert!(partials >= 3, "PartialTable 조각 셋 이상: {partials}");
}

// ── 표 앞에 글이 있어 현재 쪽 남은 자리에 안 들어가는 표 — 한글: 나누지 않음은 통째로 다음 쪽, 나머지는 쪽 경계에서 잇는다 ──

/// 첫 문단에 `lead_chars`자 글을 쓰고 둘째 문단에 `rows`×1 자리차지 표를 둔 문서.
fn doc_after_text(lead_chars: usize, rows: u16, page_break: u8) -> Vec<u8> {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().expect("blank document");
    let lead = "가나다라마바사아자차카타파하".repeat(lead_chars / 14 + 1);
    core.insert_text_native(0, 0, 0, &lead).expect("앞 글");
    core.split_paragraph_native(0, 0, lead.chars().count(), None)
        .expect("문단 나누기");
    core.create_table_ex_native(0, 1, 0, rows, 1, false, None, None)
        .expect("table");
    let control_idx = core.document().sections[0].paragraphs[1]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .expect("표 컨트롤");
    core.set_table_properties_native(
        0,
        1,
        control_idx,
        &format!(r#"{{"pageBreak":{page_break},"treatAsChar":false}}"#),
    )
    .expect("쪽 경계 속성");
    for cell in 0..rows as usize {
        core.insert_text_in_cell_native(0, 1, control_idx, cell, 0, 0, "칸 글")
            .expect("칸 글");
    }
    core.export_hwp_native().expect("export hwp")
}

#[test]
fn mid_page_none_table_that_fits_a_fresh_page_moves_whole_to_next_page() {
    // 앞 글 ≈ 절반 쪽 + 표 40행(≈ 680px — 한 쪽에는 들고 남은 자리에는 안 든다).
    let (pages, dump) = dumps(&doc_after_text(1400, 40, NONE));
    assert_eq!(
        dump.matches("PartialTable").count(),
        0,
        "나누지 않음: 표를 가르지 않는다:\n{dump}"
    );
    assert_eq!(pages, 2, "표가 통째로 다음 쪽으로 간다");
}

#[test]
fn mid_page_row_break_table_still_continues_across_the_page_boundary() {
    let (pages, dump) = dumps(&doc_after_text(1400, 40, ROW));
    assert!(
        dump.matches("PartialTable").count() >= 2,
        "나눔: 남은 자리에서 시작해 다음 쪽으로 잇는다:\n{dump}"
    );
    assert_eq!(pages, 2);
}

// ── 셀 단위로 나눔은 행 사이에서만 쪽을 잇는다 — 행 안을 자르지 않는다 ──

#[test]
fn cell_unit_never_cuts_inside_a_row_but_row_break_does() {
    // 6행 × 칸 글 1,500자(행 하나 ≈ 360px) — 쪽 경계(≈ 933px)는 셋째 행 안에 걸린다.
    let cell_unit = dumps(&doc(6, CELL, 1500)).1;
    assert!(
        cell_unit.contains("PartialTable"),
        "행 사이에서 이어진다:\n{cell_unit}"
    );
    assert!(
        !cell_unit.contains("start_cut") && !cell_unit.contains("end_cut"),
        "행 안은 자르지 않는다:\n{cell_unit}"
    );
    let row_break = dumps(&doc(6, ROW, 1500)).1;
    assert!(
        row_break.contains("end_cut"),
        "나눔은 행 안 줄을 자른다:\n{row_break}"
    );
}

// ── 한글은 밀린 «나누지 않음» 표의 host 줄과 뒤 문단을 이 쪽에 두고 표만 다음 쪽 맨 위로 보낸다 — 한컴이 저장한 242쪽 문서(뒤 문단 저장 vpos가 host 한 줄 아래이고
//    다음 쪽 첫 줄이 표 높이만큼 내려 시작한다)와 같은 모양 ──

/// 글 반 쪽 · 빈 host(표) · 뒤 문단 한 줄 — 표는 `rows`×1 자리차지.
fn doc_with_tail(lead_chars: usize, rows: u16, page_break: u8) -> Vec<u8> {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().expect("blank document");
    let lead = "가나다라마바사아자차카타파하".repeat(lead_chars / 14 + 1);
    core.insert_text_native(0, 0, 0, &lead).expect("앞 글");
    core.split_paragraph_native(0, 0, lead.chars().count(), None)
        .expect("host 문단");
    core.split_paragraph_native(0, 1, 0, None).expect("뒤 문단");
    core.insert_text_native(0, 2, 0, "표 다음 문단")
        .expect("뒤 글");
    core.create_table_ex_native(0, 1, 0, rows, 1, false, None, None)
        .expect("table");
    let control_idx = core.document().sections[0].paragraphs[1]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .expect("표 컨트롤");
    core.set_table_properties_native(
        0,
        1,
        control_idx,
        &format!(r#"{{"pageBreak":{page_break},"treatAsChar":false}}"#),
    )
    .expect("쪽 경계 속성");
    for cell in 0..rows as usize {
        core.insert_text_in_cell_native(0, 1, control_idx, cell, 0, 0, "칸 글")
            .expect("칸 글");
    }
    core.export_hwp_native().expect("export hwp")
}

#[test]
fn pushed_none_table_leaves_its_host_and_following_text_on_the_old_page() {
    let bytes = doc_with_tail(1400, 40, NONE);
    let d = reopened(&bytes);
    assert_eq!(d.page_count(), 2, "표만 다음 쪽으로 간다");
    let first = d.dump_page_items(Some(0));
    let second = d.dump_page_items(Some(1));
    assert!(!first.contains("Table"), "1쪽에는 표가 없다:\n{first}");
    assert!(first.contains("pi=2 "), "뒤 문단은 1쪽에 남는다:\n{first}");
    assert!(
        second.contains("Table") && second.contains("pi=1 "),
        "표는 2쪽 맨 위에 선다:\n{second}"
    );
}

#[test]
fn a_chain_of_pushed_none_tables_takes_one_page_each() {
    // 쪽 한 장 높이에 가까운 «나누지 않음» 표 둘이 이어지면 한 쪽에 하나씩 선다.
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().expect("blank document");
    core.insert_text_native(0, 0, 0, "표 앞 문단")
        .expect("앞 글");
    core.split_paragraph_native(0, 0, 6, None)
        .expect("host 하나");
    core.split_paragraph_native(0, 1, 0, None).expect("host 둘");
    for para in [1usize, 2] {
        core.create_table_ex_native(0, para, 0, 50, 1, false, None, None)
            .expect("table");
        let control_idx = core.document().sections[0].paragraphs[para]
            .controls
            .iter()
            .position(|c| matches!(c, Control::Table(_)))
            .expect("표 컨트롤");
        core.set_table_properties_native(
            0,
            para,
            control_idx,
            &format!(r#"{{"pageBreak":{NONE},"treatAsChar":false}}"#),
        )
        .expect("쪽 경계 속성");
        for cell in 0..50usize {
            core.insert_text_in_cell_native(0, para, control_idx, cell, 0, 0, "칸 글")
                .expect("칸 글");
        }
    }
    let d = reopened(&core.export_hwp_native().expect("export hwp"));
    let tables_per_page: Vec<usize> = (0..d.page_count())
        .map(|p| d.dump_page_items(Some(p)).matches("Table ").count())
        .collect();
    assert!(
        tables_per_page.iter().all(|&n| n <= 1),
        "쪽마다 표 하나: {tables_per_page:?}"
    );
    assert_eq!(
        tables_per_page.iter().sum::<usize>(),
        2,
        "표 둘 다 어딘가에 선다: {tables_per_page:?}"
    );
}
