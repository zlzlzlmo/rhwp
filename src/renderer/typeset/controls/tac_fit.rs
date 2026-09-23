//! 일반 TAC 표 문단의 배치 전 수용 판단.
//! 줄 상자·실측·저장 경계의 기존 선택 순서를 보존하고 상태 쓰기는 조정자에 맡긴다.

use super::super::paragraph::metrics::FormattedParagraph;
use super::super::{
    line_seg_visible_bounds_px, saved_table_bounds_fit_at_flow_tail, stored_tac_table_frame_height,
};
use super::tac_flow::TacFlowQuery;
use crate::model::{
    control::Control, paragraph::Paragraph, provenance::LayoutCompatibilityProfile,
};
use crate::renderer::{height_measurer::MeasuredTable, hwpunit_to_px, pagination::PageItem};

pub(in crate::renderer::typeset) struct TacFitPage<'a> {
    pub profile: LayoutCompatibilityProfile,
    pub current_height: f64,
    pub vpos_page_base: Option<i32>,
    pub current_items: &'a [PageItem],
}

pub(in crate::renderer::typeset) struct TacFitPlan {
    pub tac_count: usize,
    pub has_tac: bool,
    // 뒤쪽 저장 높이 cap도 동일한 편집 후 실측 결과를 소비한다.
    pub session_grown_tac_total: Option<f64>,
    /// 저장 **전** 편집으로 자란 표다(`stored_host_line_growth_hu`) — 이 뒤의 저장 사다리도 낡았다.
    pub grown_before_save: bool,
    pub advance_before_place: bool,
}

/// 가용 높이(진단 포함)는 원래 저장 경계/최종 fit 위치에서만 조회한다.
pub(super) fn prepare(
    para_idx: usize,
    para: &Paragraph,
    fmt: &FormattedParagraph,
    measured_tables: &[MeasuredTable],
    page: TacFitPage<'_>,
    available_height: impl Fn() -> f64,
    flow: TacFlowQuery<'_>,
) -> TacFitPlan {
    let dpi = flow.dpi();
    // TAC 표 카운트 및 플러시 판단
    let tac_count = para
        .controls
        .iter()
        .filter(|c| matches!(c, Control::Table(t) if flow.is_effective_tac_table(para, t, fmt)))
        .count();

    let has_tac = tac_count > 0;
    let first_line_tac_height = if tac_count == 1 && fmt.line_heights.len() > 1 {
        para.controls.iter().find_map(|ctrl| match ctrl {
            Control::Table(t)
                if flow.is_effective_tac_table(para, t, fmt)
                    && flow.tac_table_line_index(para, t, fmt) == Some(0) =>
            {
                Some(
                    fmt.line_heights
                        .first()
                        .copied()
                        .unwrap_or_else(|| fmt.line_advance(0)),
                )
            }
            _ => None,
        })
    } else {
        None
    };
    // [편집 세션] TAC 표가 셀 편집으로 자라면 저장 줄높이(표 선언 인코딩)
    // 기반 fit 은 과소가 된다 — 실측(mt)을 하한으로 써야 넘친 표가 pre-flush
    // 로 새 쪽에 간다(셀 Enter 재현: 실측이 선언 fit 으로 1쪽에 남아 하단이
    // 잘림). 저장 bounds 특례도 성장 표에는 무효다(저장 좌표는 편집 전 형상).
    // 🔴 저장 **전** 편집으로 자란 표도 같다 — 채움이 표 크기(선언 높이)를 키우고 host 줄을 다시 짜지 않은 저장본은
    // 선언 틀이 저장 줄보다 크다(한컴 저장본은 둘이 같다). 한/글은 줄을 새로 짜므로 실측으로 흘린다(맥 한글 12.30:
    // SMATEC 채움 4쪽 표 저장 줄 313.7px · 실측 374.9px — 저장 줄로 흘리면 뒤 문단이 그만큼 덜 밀린다).
    let mut grown_before_save = false;
    let session_grown_tac_total = has_tac
        .then(|| {
            para.controls.iter().enumerate().find_map(|(ci, ctrl)| {
                let Control::Table(t) = ctrl else { return None };
                if !flow.is_effective_tac_table(para, t, fmt) {
                    return None;
                }
                let declared = hwpunit_to_px(t.common.height as i32, dpi);
                let measured = measured_tables
                    .iter()
                    .find(|m| m.para_index == para_idx && m.control_index == ci)?;
                let grown_in_session =
                    flow.session_edited() && measured.total_height > declared + 8.0;
                let predates = crate::renderer::typeset::stored_host_line_growth_hu(
                    para,
                    t,
                    flow.tac_table_line_index(para, t, fmt),
                    ci,
                )
                .is_some();
                grown_before_save |= predates;
                (grown_in_session || predates).then_some(measured.total_height)
            })
        })
        .flatten();
    // 실제 TAC 배치가 사용하는 소유 줄 상자는 바깥여백을 이미 포함한다.
    // pre-flush에서 fmt와 여백을 다시 더하면 실제로 들어가는 표를 먼저 이월한다.
    let owned_single_tac_frame = (page.profile.hwpx_stored_layout()
        && tac_count == 1
        && fmt.line_heights.len() == 1)
        .then(|| {
            para.controls.iter().enumerate().find_map(|(ci, control)| {
                let Control::Table(table) = control else {
                    return None;
                };
                crate::renderer::composer::owned_rowbreak_tac_height(para, ci).filter(|height| {
                    i64::from(*height)
                        >= i64::from(table.common.height)
                            + i64::from(table.outer_margin_top)
                            + i64::from(table.outer_margin_bottom)
                })
            })
        })
        .flatten()
        .map(|height| hwpunit_to_px(height, dpi));
    let height_for_fit = if let Some(height) = owned_single_tac_frame {
        let base = height + fmt.spacing_before;
        session_grown_tac_total.map_or(base, |grown| base.max(grown))
    } else if has_tac {
        // 글자처럼 취급되는 표는 **바깥 여백(위·아래)까지 쪽 예산을 차지**한다.
        // 한컴 저장 lineseg 의 vertsize 가 `표 선언높이 + outMargin.top + outMargin.bottom`
        // 이다(본 문서 TAC 개체 18/18 일치, 2248+283+283=2814). 이 항이 빠져 쪽마다
        // 566 HU 씩 덜 쌓였고, 소제목 표가 앞 쪽 바닥에 남아 이후 쪽이 통째로 밀렸다.
        // 소유 줄이 상하 여백까지 담는 경로는 위에서 한 번만 계상한다.
        // 그 증거가 없는 저장 줄은 기존 수용 판정의 여백 보충을 유지한다.
        // 🔴 rhwp 가 다시 조판한 줄(합성 태그 — «저장 조판 없음»)은 줄 높이와 형식 단계(`tac_outer_margin_v_px`)가
        // 이미 담은 몫을 빼고 남은 만큼만 더한다 — 셋이 겹쳐 3.8px 를 세 번 셌다(맥 한글 12.30: 도약 채움 일반현황
        // 둘째 표가 2쪽 끝에 들어가 8쪽 · rhwp 9쪽). 한컴 저장 줄은 위 주석의 보충을 그대로 둔다(상류 기준선 계약).
        let rhwp_composed_lines = crate::renderer::para_has_no_stored_line_segs(para);
        let tallest_line = fmt.line_heights.iter().copied().fold(0.0f64, f64::max);
        let tac_outer_margin_px: f64 = para
            .controls
            .iter()
            .filter_map(|ctrl| match ctrl {
                Control::Table(t) if flow.is_effective_tac_table(para, t, fmt) => {
                    Some(if rhwp_composed_lines {
                        super::super::paragraph::format::tac_outer_margin_deficit_px(
                            t,
                            tallest_line + fmt.tac_outer_margin_v_px,
                            dpi,
                        )
                    } else {
                        crate::renderer::hwpunit_to_px(
                            i32::from(t.common.margin.top) + i32::from(t.common.margin.bottom),
                            dpi,
                        )
                    })
                }
                _ => None,
            })
            .fold(0.0f64, f64::max);
        let base = first_line_tac_height.unwrap_or(fmt.height_for_fit) + tac_outer_margin_px;
        session_grown_tac_total.map_or(base, |grown| base.max(grown))
    } else {
        fmt.total_height
    };
    let saved_single_tac_bottom_fits = if has_tac
        && tac_count <= 1
        && session_grown_tac_total.is_none()
    {
        para.controls
            .iter()
            .find_map(|ctrl| match ctrl {
                Control::Table(table) if flow.is_effective_tac_table(para, table, fmt) => Some((
                    flow.tac_table_line_index(para, table, fmt).unwrap_or(0),
                    stored_tac_table_frame_height(table, dpi, height_for_fit),
                )),
                _ => None,
            })
            .and_then(|(line_idx, frame_height)| {
                para.line_segs.get(line_idx).and_then(|seg| {
                    line_seg_visible_bounds_px(seg, page.vpos_page_base.unwrap_or(0), dpi)
                        .map(|bounds| (bounds, frame_height))
                })
            })
            .is_some_and(|(bounds, frame_height)| {
                saved_table_bounds_fit_at_flow_tail(
                    bounds,
                    page.current_height,
                    available_height(),
                    frame_height,
                )
            })
    } else {
        false
    };
    // [#2311] 단일 TAC 표가 후행 줄(ctrl 1:1 lineseg, vpos==0 저장 리셋)에 있고
    // 선행 줄이 전부 TAC 그림/도형이면, 표는 아래 #1152 intra-para reset 가드가
    // 자체적으로 새 쪽 이동한다. 이때 pre-flush 를 문단 전체 높이로 판정하면
    // 잔여 공간에 들어가는 선행 전면 그림까지 통째로 밀려 한글 대비 +1쪽씩
    // 벌어진다 (10k r15 156744475: 붙임 포스터+차기 붙임 헤더 표 문단 ×2 →
    // rhwp 5쪽 vs 한글 3쪽, 저장 ls[0] vpos=5435 는 같은 쪽 배치를 명시).
    // 리셋 이전 줄들의 높이만 fit 기준으로 삼는다.
    let pre_reset_height_for_fit = if has_tac
        && tac_count == 1
        && first_line_tac_height.is_none()
        && para.text.is_empty()
        && para.line_segs.len() == para.controls.len()
    {
        para.controls
            .iter()
            .position(
                |c| matches!(c, Control::Table(t) if flow.is_effective_tac_table(para, t, fmt)),
            )
            .filter(|&ti| {
                ti > 0
                    && ti <= fmt.line_heights.len()
                    && para.line_segs.get(ti).map(|s| s.vertical_pos) == Some(0)
                    && para.controls[..ti].iter().all(|c| match c {
                        Control::Picture(p) => p.common.treat_as_char,
                        Control::Shape(s) => s.common().treat_as_char,
                        _ => false,
                    })
            })
            .map(|ti| (0..ti).map(|li| fmt.line_advance(li)).sum::<f64>())
    } else {
        None
    };
    let height_for_fit = pre_reset_height_for_fit.unwrap_or(height_for_fit);

    // 넘치면 flush (단일 TAC 표만)
    let advance_before_place = page.current_height + height_for_fit > available_height()
        && !page.current_items.is_empty()
        && has_tac
        && tac_count <= 1
        && !saved_single_tac_bottom_fits;
    TacFitPlan {
        tac_count,
        has_tac,
        session_grown_tac_total,
        grown_before_save,
        advance_before_place,
    }
}
