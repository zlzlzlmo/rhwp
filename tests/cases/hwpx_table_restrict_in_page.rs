//! hwpx로 연 표의 «쪽 영역 안으로 제한»(`restrictInPage`)·«겹침 허용»(`allowOverlap`)은 `hp:pos`의
//! `flowWithText`·`allowOverlap` 그대로 읽혀야 한다.
//!
//! 종전: hwpx 리더(`materialize_hwpx_table_attrs`)가 표 속성(`table.attr`)에 bit0(글자처럼)만 옮겨,
//! `getTableProperties`가 bit13·bit14를 읽는 두 값이 hwpx 문서에서 늘 false였다 — 조판은
//! `common.flow_with_text`(참)를 쓰므로 같은 표가 hwp로 열면 참 · hwpx로 열면 거짓으로 갈렸다.
#![cfg(not(target_arch = "wasm32"))]

use rhwp::wasm_api::HwpDocument;
use serde_json::Value;

fn props(doc: &HwpDocument, para: u32, control: u32) -> Value {
    serde_json::from_str(
        &doc.get_table_properties(0, para, control)
            .expect("table properties"),
    )
    .expect("JSON")
}

#[test]
fn hwpx_table_restrict_in_page_and_allow_overlap_survive_hwpx_reopen() {
    let mut doc = HwpDocument::create_empty();
    doc.create_blank_document_native().expect("blank");
    let created: Value =
        serde_json::from_str(&doc.create_table_native(0, 0, 0, 2, 2).expect("table"))
            .expect("JSON");
    let para = created["paraIdx"].as_u64().expect("paraIdx") as u32;
    let control = created["controlIdx"].as_u64().expect("controlIdx") as u32;
    for (restrict, overlap) in [(true, false), (false, true), (true, true)] {
        doc.set_table_properties_native(
            0,
            para as usize,
            control as usize,
            &format!(r#"{{"restrictInPage":{restrict},"allowOverlap":{overlap}}}"#),
        )
        .expect("set");
        let before = props(&doc, para, control);
        assert_eq!(before["restrictInPage"], restrict);
        assert_eq!(before["allowOverlap"], overlap);

        let hwpx = doc.export_hwpx_native().expect("hwpx");
        let reopened = HwpDocument::from_bytes(&hwpx).expect("reopen hwpx");
        let after = props(&reopened, para, control);
        assert_eq!(
            after["restrictInPage"], restrict,
            "hwpx reopen restrictInPage"
        );
        assert_eq!(after["allowOverlap"], overlap, "hwpx reopen allowOverlap");
    }
}
