//! 글자처럼 취급한 «나눔» 다중 행 표가 한 쪽보다 크게 자라면(채움·편집으로 칸 글이 늘어 행이 선언 높이를 넘은 표) 표째 다음 쪽으로
//! 밀거나 쪽 밖으로 넘쳐 그리지 않고 쪽 경계에서 이어 그린다 — 한글이 그리는 모양이다(위비즈 248차 T7 · rhwp #7288).
//!
//! 재현: 2×1 표(머리 행 + 큰 칸 행) · 글자처럼 취급 · 쪽 나눔 «나눔»(2) · 칸 행을 한 쪽보다 크게 키우고(저장 host 줄은 낡은 채)
//! 칸에 긴 글을 넣는다. 종전 엔진은 표 하나를 한 덩어리 개체(`Table`)로 놓아 한 쪽에 몰아 그렸다.

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::wasm_api::HwpDocument;

/// 2×1 글자처럼 표를 가진 문서 바이트. `content_row_height_hu`는 칸 행(둘째 행)의 선언 높이, `chars`는 칸에 넣을 글자 수.
fn tac_two_row_table_doc(page_break: u8, content_row_height_hu: u32, chars: usize) -> Vec<u8> {
    let mut core = DocumentCore::new_empty();
    core.create_blank_document_native().expect("blank document");
    core.create_table_ex_native(0, 0, 0, 2, 1, true, None, None)
        .expect("2x1 inline table");
    let control_idx = core.document().sections[0].paragraphs[0]
        .controls
        .iter()
        .position(|c| matches!(c, Control::Table(_)))
        .expect("표 컨트롤");
    core.set_table_properties_native(
        0,
        0,
        control_idx,
        &format!(r#"{{"pageBreak":{page_break},"treatAsChar":true}}"#),
    )
    .expect("쪽 나눔 속성");
    core.insert_text_in_cell_native(
        0,
        0,
        control_idx,
        1,
        0,
        0,
        &"가나다라마바사아자차카타파하".repeat(chars / 14 + 1),
    )
    .expect("칸 글");
    core.set_cell_properties_native(
        0,
        0,
        control_idx,
        1,
        &format!(r#"{{"height":{content_row_height_hu}}}"#),
    )
    .expect("칸 행 높이");
    core.export_hwp_native().expect("export hwp")
}

fn page_dump(bytes: &[u8], page_idx: u32) -> String {
    HwpDocument::from_bytes(bytes)
        .unwrap_or_else(|e| panic!("parse: {e:?}"))
        .dump_page_items(Some(page_idx))
}

fn page_count(bytes: &[u8]) -> u32 {
    HwpDocument::from_bytes(bytes)
        .unwrap_or_else(|e| panic!("parse: {e:?}"))
        .page_count()
}

#[test]
fn oversized_rowbreak_inline_table_continues_on_next_page() {
    // 칸 행 100,000 HU ≈ 1,333px — A4 본문(≈ 933px)보다 크다.
    let bytes = tac_two_row_table_doc(2, 100_000, 3000);
    let first = page_dump(&bytes, 0);
    assert!(
        first.contains("PartialTable") && first.contains("cont=false"),
        "첫 쪽은 표의 앞부분이어야 한다(표째 한 쪽에 몰아 그리지 않는다):\n{first}"
    );
    let second = page_dump(&bytes, 1);
    assert!(
        second.contains("PartialTable") && second.contains("cont=true"),
        "둘째 쪽은 표의 이어진 부분이어야 한다:\n{second}"
    );
    assert_eq!(page_count(&bytes), 2, "표가 두 쪽에 걸쳐 끝난다");
}

#[test]
fn inline_table_that_fits_one_page_is_not_split() {
    // 칸 행 40,000 HU ≈ 533px — 한 쪽에 든다. 종전 경로(Table 한 덩어리)를 그대로 탄다.
    let bytes = tac_two_row_table_doc(2, 40_000, 400);
    let first = page_dump(&bytes, 0);
    assert!(
        !first.contains("PartialTable"),
        "한 쪽에 드는 표는 가르지 않는다:\n{first}"
    );
    assert_eq!(page_count(&bytes), 1);
}

#[test]
fn non_breaking_inline_table_is_untouched() {
    // 나누지 않음(0)은 어느 엔진에서도 못 가른다 — 종전과 같다.
    let bytes = tac_two_row_table_doc(0, 100_000, 3000);
    assert!(!page_dump(&bytes, 0).contains("PartialTable"));
}
