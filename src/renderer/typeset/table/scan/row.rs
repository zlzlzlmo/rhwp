//! 일반 행·rowspan 행의 요구 높이와 잔여 밴드 Query. 예산 수용과 상태 반영은 부모 소유다.

use super::RowBlockQuery;
use crate::model::control::Control;
use crate::model::table::Table;
use crate::renderer::height_measurer::MeasuredTable;
use crate::renderer::layout::table_layout::RowCutResult;
use crate::renderer::typeset::MIN_TOP_KEEP_PX;

pub(in crate::renderer::typeset) struct RowScanQuery<'a> {
    pub(in crate::renderer::typeset) rows: &'a RowBlockQuery<'a>,
    pub(in crate::renderer::typeset) r: usize,
    pub(in crate::renderer::typeset) cursor_row: usize,
    pub(in crate::renderer::typeset) start_cut: &'a [usize],
    pub(in crate::renderer::typeset) start_row_height_override: Option<f64>,
}

pub(in crate::renderer::typeset) struct RowBandShape {
    pub(in crate::renderer::typeset) has_prior_rowspan_cover: bool,
    pub(in crate::renderer::typeset) row_has_nested: bool,
}

pub(in crate::renderer::typeset) struct RowBandProbe {
    pub(in crate::renderer::typeset) probe: RowCutResult,
    pub(in crate::renderer::typeset) visible_height: f64,
}

impl RowScanQuery<'_> {
    /// 앞 행이 실제 수용한 높이를 차감한다. 원본 측정 행과 컷용 행 높이를 합치지 않는다.
    pub(in crate::renderer::typeset) fn required_height(
        &self,
        height: f64,
        consumed: f64,
        cs_before: f64,
    ) -> f64 {
        let Self {
            rows,
            r,
            cursor_row,
            start_cut,
            start_row_height_override,
        } = *self;
        let RowBlockQuery {
            layout_engine,
            mt,
            table,
            styles,
            ..
        } = *rows;
        layout_engine
            .straddle_continuation_demand(
                table,
                r,
                cursor_row,
                start_cut,
                start_row_height_override,
                &mt.row_heights,
                styles,
                (r + 1, true),
            )
            .map_or(height, |need| height.max(need - consumed - cs_before))
    }

    pub(in crate::renderer::typeset) fn whole_row_height(
        &self,
        row_start_cut: &[usize],
        whole_row_fit_h: &[f64],
    ) -> f64 {
        let Self { rows, r, .. } = *self;
        let RowBlockQuery {
            layout_engine,
            table,
            styles,
            ..
        } = *rows;
        if row_start_cut.is_empty() {
            whole_row_fit_h[r]
        } else {
            // 연속분 cursor_row — 시작 컷 적용. row_cut_content_height 가
            // 셀별 (content+pad) 행 max 를 반환(분할 행이므로 cell.height
            // 강제 없음).
            layout_engine.row_cut_content_height(table, r, row_start_cut, &[], styles)
        }
    }

    pub(in crate::renderer::typeset) fn band_shape(&self) -> RowBandShape {
        let Self { rows, r, .. } = *self;
        let table = rows.table;
        let has_prior_rowspan_cover = table.cells.iter().any(|c| {
            c.row_span > 1 && (c.row as usize) < r && r < c.row as usize + c.row_span as usize
        });
        let row_has_nested = table.cells.iter().any(|c| {
            c.row as usize == r
                && c.paragraphs.iter().any(|p| {
                    p.controls
                        .iter()
                        .any(|ctrl| matches!(ctrl, Control::Table(_)))
                })
        });

        RowBandShape {
            has_prior_rowspan_cover,
            row_has_nested,
        }
    }

    /// 부모의 기존 guard 안에서만 실행한다. 컷 조회와 실제 표시 높이 조회 순서를 유지한다.
    pub(in crate::renderer::typeset) fn probe_band(
        &self,
        row_start_cut: &[usize],
        rest: f64,
    ) -> RowBandProbe {
        let Self { rows, r, .. } = *self;
        let RowBlockQuery {
            layout_engine,
            table,
            styles,
            ..
        } = *rows;
        let padding =
            layout_engine.row_remaining_visible_padding_height(table, r, row_start_cut, styles);
        let content_budget = (rest - padding).max(0.0);
        let probe = layout_engine.advance_row_cut(table, r, row_start_cut, content_budget, styles);
        let visible_height =
            layout_engine.row_cut_content_height(table, r, row_start_cut, &probe.end_cut, styles);

        RowBandProbe {
            probe,
            visible_height,
        }
    }
}

/// 빈 띠 행을 가르는 자리는 본문 아래보다 이만큼 위다(HWPUNIT). 맥 한글 12.30 실측: 경북 판로지원 6×1 표를
/// 위아래로 옮긴 사본 넷(−19.2·0·+24·+48pt)·칸 여백 셋·쪽 아래 여백 셋(15·15.1·20mm) 모두 자르는 선이
/// 본문 아래 − 1.00pt(±0.01)에 선다.
pub(in crate::renderer::typeset) const EMPTY_BAND_CUT_BOTTOM_RESERVE_HU: i32 = 100;

/// 빈 띠를 가른 뒤 남은 띠가 이보다 짧으면 다음 쪽에 그리지 않고 버린다(HWPUNIT). 맥 한글 12.30 실측: f5a3062b 5×2 표
/// 3행 선언 높이 변형 — 남은 띠 4.2 · 7.7 · 10.0 · 12.2 · 12.5 · 12.8pt 는 버리고 12.9 · 13.0 · 13.2 · 15.2 · 18.2pt 부터
/// 그린다. 경북 6×1 표(10pt 칸)도 12.8pt 버림 · 13.0pt 그림 — 칸 글자 크기와 무관한 상수다.
pub(in crate::renderer::typeset) const EMPTY_BAND_MIN_CARRIED_TAIL_HU: i32 = 1280;

/// 나눔(RowBreak) 표에서 선언 행 높이가 칸 글(줄 + 위아래 여백)보다 [`MIN_TOP_KEEP_PX`] 넘게 큰 행 — 글 아래 빈
/// 띠가 행 높이를 정한다. 맥 한글 12.30 은 이런 행을 쪽 끝에서 띠째 가르고 남은 띠를 다음 쪽 첫머리에 그린다
/// (경북 판로지원 6×1 표 빈 칸 386.9pt: 2쪽 188.6pt + 3쪽 198.2pt). 칸에 글 밖 개체(글자처럼 취급하지 않는 그림·
/// 도형·표·수식)가 있으면 띠를 그 개체가 채울 수 있어 제외한다(간장 보고서 그림 8). 글자처럼 놓인 개체는 줄 높이에
/// 들어 있어 띠와 무관하다(f5a3062b 3쪽 5×2 표 3행 — 칸 속 글자처럼 표 · 맥은 쪽 끝 − 100HU 에서 가른다).
pub(in crate::renderer::typeset) fn row_is_declared_empty_band(
    mt: &MeasuredTable,
    table: &Table,
    r: usize,
) -> bool {
    let Some(&row_h) = mt.row_heights.get(r) else {
        return false;
    };
    let mut cells = mt
        .cells
        .iter()
        .filter(|c| c.row == r && c.row_span == 1)
        .peekable();
    mt.allows_row_break_split()
        && cells.peek().is_some()
        && cells.all(|c| {
            c.total_content_height + c.padding_top + c.padding_bottom + MIN_TOP_KEEP_PX < row_h
        })
        && !row_has_out_of_line_control(table, r)
}

/// 선언 행 높이가 칸 글(줄 + 위아래 여백)보다 큰 몫 — 글 아래 빈 띠의 높이(px). 행 높이가 글로 정해지면 0 이다.
pub(in crate::renderer::typeset) fn row_declared_blank_band(mt: &MeasuredTable, r: usize) -> f64 {
    let Some(&row_h) = mt.row_heights.get(r) else {
        return 0.0;
    };
    let content = mt
        .cells
        .iter()
        .filter(|c| c.row == r && c.row_span == 1)
        .map(|c| c.total_content_height + c.padding_top + c.padding_bottom)
        .fold(f64::NAN, f64::max);
    if content.is_nan() {
        0.0
    } else {
        (row_h - content).max(0.0)
    }
}

/// 행 칸에 글 밖 개체(글자처럼 취급하지 않는 그림·도형·표·수식)가 있는가 — 선언 높이의 띠를 그 개체가 채울 수 있다.
pub(in crate::renderer::typeset) fn row_has_out_of_line_control(table: &Table, r: usize) -> bool {
    table.cells.iter().any(|cell| {
        cell.row as usize == r
            && cell
                .paragraphs
                .iter()
                .any(|p| p.controls.iter().any(control_floats_out_of_line))
    })
}

/// 글자처럼 취급하지 않는 개체 — 줄 밖에 떠서 칸 공간을 따로 차지한다.
fn control_floats_out_of_line(control: &Control) -> bool {
    match control {
        Control::Picture(picture) => !picture.common.treat_as_char,
        Control::Shape(shape) => !shape.common().treat_as_char,
        Control::Table(table) => !table.common.treat_as_char,
        Control::Equation(equation) => !equation.common.treat_as_char,
        Control::Form(form) => !form.common.treat_as_char,
        _ => false,
    }
}

pub(in crate::renderer::typeset) fn retains_blank_tail(
    probe: &RowCutResult,
    visible_height: f64,
    rest: f64,
) -> bool {
    let retained_blank_tail = (rest - visible_height).max(0.0);
    probe.fully_consumed && visible_height <= rest + 0.5 && retained_blank_tail >= MIN_TOP_KEEP_PX
}
