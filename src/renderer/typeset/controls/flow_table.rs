//! 일반/TAC 표 하나의 포맷·배치 선택과 후행 표 지연 등록 조정.
//! 배치 알고리즘은 기존 엔진 경로를 호출하며 각주 수집·반복 중단은 부모가 소유한다.
use super::super::paragraph::metrics::FormattedParagraph;
use super::super::{
    preceding_stored_vpos, stored_host_line_growth_hu, FormattedTable, TypesetEngine, TypesetState,
};
use crate::model::{paragraph::Paragraph, table::Table};
use crate::renderer::{
    composer::ComposedParagraph,
    float_placement::{signed_hwpunit, FloatLaneSet},
    height_measurer::MeasuredTable,
    hwpunit_to_px,
    style_resolver::ResolvedStyleSet,
};

/// 소유 문단의 기존 입력만 빌린다. 상태와 float lane은 별도 가변 인자로 전달한다.
pub(in crate::renderer::typeset) struct FlowTableInput<'a> {
    pub para_idx: usize,
    pub ctrl_idx: usize,
    pub para: &'a Paragraph,
    pub table: &'a Table,
    pub fmt: &'a FormattedParagraph,
    pub measured_tables: &'a [MeasuredTable],
    pub styles: &'a ResolvedStyleSet,
    pub composed: Option<&'a ComposedParagraph>,
    pub next_para: Option<&'a Paragraph>,
    pub tac_count: usize,
    pub first_placed_table: Option<usize>,
    pub last_placed_table: Option<usize>,
    pub para_start_height: f64,
    pub paragraphs_all: &'a [Paragraph],
    pub composed_all: &'a [ComposedParagraph],
    pub ctrl_order: &'a [usize],
    pub order_pos: usize,
}

/// strict fit 경로의 선행 배타 영역 소비에 필요한 높이만 조회한다.
fn natural_top_lead(table: &Table, ft: &FormattedTable, dpi: f64) -> Option<f64> {
    if ft.strict_following_plain_text_fit {
        Some(
            hwpunit_to_px(signed_hwpunit(table.common.vertical_offset).max(0), dpi)
                + hwpunit_to_px(table.outer_margin_top as i32, dpi),
        )
    } else {
        None
    }
}

pub(in crate::renderer::typeset) fn place(
    engine: &TypesetEngine,
    st: &mut TypesetState,
    input: FlowTableInput<'_>,
    para_float_lanes: &mut FloatLaneSet,
) -> bool {
    let FlowTableInput {
        para_idx,
        ctrl_idx,
        para,
        table,
        fmt,
        measured_tables,
        styles,
        composed,
        next_para,
        tac_count,
        first_placed_table,
        last_placed_table,
        para_start_height,
        paragraphs_all,
        composed_all,
        ctrl_order,
        order_pos,
    } = input;
    let mut break_after_current_table = false;
    let is_column_top = st.flow_table_column_top();
    let ft = engine.format_table(
        para,
        para_idx,
        ctrl_idx,
        table,
        measured_tables,
        styles,
        composed,
        next_para,
        is_column_top,
    );

    let issue2439_para_start_height =
        st.prepare_flow_table_anchor(natural_top_lead(table, &ft, engine.dpi), para_start_height);

    let mt = measured_tables
        .iter()
        .find(|mt| mt.para_index == para_idx && mt.control_index == ctrl_idx);
    let is_first_placed = first_placed_table == Some(ctrl_idx);
    let is_last_placed = last_placed_table == Some(ctrl_idx);
    let tac_to_block = engine.is_effective_tac_table(para, table, fmt)
        && tac_oversized_rowbreak_to_block(engine, st, table, mt, para, fmt, ctrl_idx, tac_count);
    if engine.is_effective_tac_table(para, table, fmt) && !tac_to_block {
        engine.typeset_tac_table(
            st,
            para_idx,
            ctrl_idx,
            para,
            table,
            &ft,
            fmt,
            tac_count,
            is_first_placed,
            is_last_placed,
            styles,
            preceding_stored_vpos(paragraphs_all, para_idx),
        );
    } else if !tac_to_block
        && engine.try_typeset_empty_para_float_table(
            st,
            para_idx,
            ctrl_idx,
            para,
            table,
            &ft,
            composed,
            next_para,
            styles,
            para_start_height,
            para_float_lanes,
        )
    {
        // Empty host para-float table placed by horizontal lane reservation.
    } else {
        let pages_before_block_table = st.flow_table_page_count();
        engine.typeset_block_table(
            st,
            para_idx,
            ctrl_idx,
            para,
            table,
            &ft,
            fmt,
            mt,
            styles,
            issue2439_para_start_height,
            // [Task #1860] 비지연 경로: para_start_height 가 곧 참 para_start.
            issue2439_para_start_height,
            is_first_placed,
            is_last_placed,
            paragraphs_all,
            composed_all,
        );
        let deferred_query =
            super::deferred::CoanchoredTableQuery::new(para, fmt, engine.tac_flow_query());
        let (page_count, current_items) = st.coanchored_table_page();
        if deferred_query.should_defer_remaining_coanchored_rowbreak_tables(
            table,
            page_count,
            current_items,
            pages_before_block_table,
        ) {
            let deferred = deferred_query.remaining_controls(
                para_idx,
                ctrl_order,
                order_pos,
                first_placed_table,
                last_placed_table,
                para_start_height,
            );
            if !deferred.is_empty() {
                st.enqueue_deferred_table_controls(deferred);
                break_after_current_table = true;
            }
        }
    }

    break_after_current_table
}

/// 글자처럼 취급한 «나눔» 다중 행 표가 **한 쪽 본문보다 커서 어디에도 통째로 못 앉으면** 블록 표 경로(행 단위·행 안 분할)로 보낸다.
///
/// 한글은 이런 표를 현재 쪽에서 시작해 쪽 경계에서 이어 그린다. TAC 경로는 표를 한 덩어리 개체로만 다뤄 다음 쪽으로 밀고 쪽 밖으로
/// 넘쳐 그린다(rhwp #7288). 조건은 일부러 좁다:
/// - 한 쪽보다 큰 표만 — 한 쪽에 들어가는 표는 종전 TAC 경로(표째 다음 쪽)가 한글 저장 조판과 맞는다.
/// - **저장 host 줄보다 표 선언 높이가 자란 표만**(`stored_host_line_growth_hu`) — 편집·채움으로 칸 글이 늘어 행(선언 높이)을 키웠는데 host 줄은 옛 값 그대로인 표다.
///   한컴이 저장한 표는 선언 높이와 host 줄이 맞는다(쪽을 직접 가른 표 포함 — 그 저장 조판이 쪽 나눔을 증언하므로 손대지 않는다).
///   rhwp가 측정한 높이가 아니라 **파일에 적힌 선언 높이**로 잰다 — 측정은 글자 폭 차이로 한컴 문서에서도 10% 남짓 크게 나온다(실측: 6개 회귀 문서가 이 차이로 걸렸다).
#[allow(clippy::too_many_arguments)]
fn tac_oversized_rowbreak_to_block(
    engine: &TypesetEngine,
    st: &TypesetState,
    table: &Table,
    mt: Option<&MeasuredTable>,
    para: &Paragraph,
    fmt: &FormattedParagraph,
    ctrl_idx: usize,
    tac_count: usize,
) -> bool {
    use crate::model::table::TablePageBreak;
    if !table.common.treat_as_char
        || table.row_count < 2
        || tac_count != 1
        || !para.text.trim().is_empty()
        || !matches!(
            table.page_break,
            TablePageBreak::RowBreak | TablePageBreak::CellBreak
        )
    {
        return false;
    }
    let Some(mt) = mt else { return false };
    mt.total_height > st.available_height() + 0.5
        && stored_host_line_growth_hu(
            para,
            table,
            engine.tac_table_line_index(para, table, fmt),
            ctrl_idx,
        )
        .is_some()
}
