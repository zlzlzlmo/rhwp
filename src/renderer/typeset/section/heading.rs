//! 구역 문단 처리의 keep_heading_with_following_block 단계. 조건·예약·발행 순서를 유지한다.
use crate::renderer::composer::ComposedParagraph;
use crate::renderer::style_resolver::ResolvedStyleSet;
use crate::renderer::typeset::{PageItem, Paragraph, TypesetEngine, TypesetState};
impl TypesetEngine {
    /// 한/글 «다음 문단과 함께»(문단 모양) — 이 문단이 다음 문단의 **첫 줄**과 같은 쪽에 서야 한다.
    ///
    /// 맥 한글 12.30 쓸기 실측(채움 줄 34~42 · 한 줄 제목 + 7줄 본문, 2026-09-23): 본문 첫 줄이 이 쪽에 들어가면
    /// 본문이 갈려도 제목은 그대로다(1/6 분할까지 같음). 본문 첫 줄이 못 들어가면 제목이 본문과 함께 다음 쪽으로
    /// 간다 — 보호가 없으면 제목만 쪽 끝에 홀로 남는 자리다.
    /// 사슬(«다음 문단과 함께»가 잇달아 걸린 문단들)은 **짝마다** 잰다 — 사슬 머리부터 쌓아 가다 어느 문단의 첫 조각이
    /// 이 쪽에 못 서면 그 앞 문단이 따라 넘어가고, 그 앞도 따라가 결국 머리부터 넘어간다(맥 한글 12.30: 초창패 채움
    /// «< 사업비 집행 계획 >» → 안내 표 → 빈 문단 → 표 사슬이 한 쪽보다 길어도 캡션이 표와 함께 다음 쪽).
    /// 첫 조각은 그 문단의 보호 규칙이 정한다 — 문단 보호면 통째, 외톨이줄 보호면 두 줄(세 줄 이하는 통째)이다(맥 한글:
    /// 예창패 채움 «◦ 소비자…» 다음 두 줄 문단은 첫 줄만 들어가는 자리에서 소제목이 함께 넘어간다).
    /// 넘어가도 새 쪽에 그 조각까지 못 서면 그대로 둔다. 이 쪽에 이미 무엇이 있을 때만 넘긴다.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn keep_paragraph_with_next(
        &self,
        st: &mut TypesetState,
        para_idx: usize,
        para: &Paragraph,
        paragraphs: &[Paragraph],
        composed: &[ComposedParagraph],
        styles: &ResolvedStyleSet,
    ) {
        let style_of = |p: &Paragraph| styles.para_styles.get(p.para_shape_id as usize);
        let keeps = |p: &Paragraph| style_of(p).is_some_and(|style| style.keep_with_next);
        if !keeps(para)
            || st.current_items.is_empty()
            || st.col_count != 1
            || st.wrap_around_cs >= 0
            || (para_idx > 0 && keeps(&paragraphs[para_idx - 1]))
        {
            return;
        }
        let col_w = st
            .layout
            .column_areas
            .get(st.current_column as usize)
            .map(|a| a.width)
            .unwrap_or(st.layout.body_area.width);
        let mut need = 0.0;
        let mut j = para_idx;
        while let Some(p) = paragraphs.get(j) {
            let fmt = self.format_paragraph(p, composed.get(j), styles, Some(col_w));
            if j > para_idx {
                let first = fmt.spacing_before + first_fragment_height(&fmt, style_of(p));
                if st.current_height + need + first > st.available_height() {
                    if need + first <= st.base_available_height() {
                        st.advance_column_or_new_page();
                    }
                    return;
                }
            }
            if !(keeps(p) && j + 1 < paragraphs.len()) {
                return;
            }
            need += fmt.total_height;
            j += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn keep_heading_with_following_block(
        &self,
        st: &mut TypesetState,
        para_idx: usize,
        para: &Paragraph,
        paragraphs: &[Paragraph],
    ) {
        // [Task #404] heading-orphan 패턴 보정 (vpos 기반).
        // 현재 paragraph 가 누적 height 로는 fit 하지만 HWP vpos 기준 페이지 한계를
        // 넘고, 다음 substantial block(Table/Shape/큰 paragraph)이 잔여 영역에 들어
        // 가지 않을 때 → 현재 paragraph 를 다음 페이지로 push 하여 heading + 후속
        // 블록을 같은 페이지에 배치.
        //
        // 조건 (모두 AND):
        //   A) current_items 비어있지 않음 (페이지 첫 item 자기참조 회피)
        //   B) 단일 단 + wrap-around zone 아님 (multi-column / wrap 의미 차이 회피)
        //   C) 누적 height 로 fit
        //   D) vpos overflow > 1mm (283 HU)
        //   E) 다음 paragraph 의 height 가 substantial (>30px ≈ 8mm) AND 잔여 영역에
        //      들어가지 않음
        //
        // Stage 1 진단 로그 분석으로 false positive 41건 → 1건(pi=83)으로 축소.
        // page_top_vpos 는 current_items 의 첫 item 으로 즉시 계산한다(`page_top_stored_vpos`).
        // (TypesetState 필드 추적은 typeset_paragraph 내부 페이지 flush 와 동기 안 됨).
        if !st.current_items.is_empty() && st.wrap_around_cs < 0 && st.col_count == 1 {
            let page_top_vpos_opt = page_top_stored_vpos(&st.current_items, paragraphs);
            if let (Some(first_seg), Some(page_top_vpos)) =
                (para.line_segs.first(), page_top_vpos_opt)
            {
                let body_h_hu =
                    crate::renderer::px_to_hwpunit(st.layout.body_area.height, self.dpi);
                let para_h_px: f64 = para
                    .line_segs
                    .iter()
                    .map(|s| {
                        crate::renderer::hwpunit_to_px(
                            s.line_height.saturating_add(s.line_spacing),
                            self.dpi,
                        )
                    })
                    .sum();
                let para_h_hu = crate::renderer::px_to_hwpunit(para_h_px, self.dpi);
                // [Task #643] vpos_end 는 마지막 줄의 bottom (vpos + lh) 기준.
                // para_h_px 누적은 트레일링 line_spacing 까지 포함하여 ~10-12 HU 과대.
                // HWP 가 페이지 끝에서 트레일링 ls 를 고려하지 않고 lh 만 fit 검사하는
                // 시멘틱 정합 (pi=39 page 3 fits 케이스).
                // 손상 입력의 거대한 vpos/height 로 i32 덧셈이 오버플로(패닉)하지
                // 않도록 saturating — 정상값에선 동일, 손상값은 i32::MAX 로 포화.
                let vpos_end = para
                    .line_segs
                    .last()
                    .map(|s| s.vertical_pos.saturating_add(s.line_height))
                    .unwrap_or(first_seg.vertical_pos.saturating_add(para_h_hu));
                let page_bottom_vpos = page_top_vpos.saturating_add(body_h_hu);

                let avail = st.available_height();
                let current_fits = st.current_height + para_h_px <= avail;
                let vpos_overflow = vpos_end > page_bottom_vpos + 283; // 1mm 안전여유

                let next_h_px: f64 = paragraphs
                    .get(para_idx + 1)
                    .map(|p| {
                        p.line_segs
                            .iter()
                            .map(|s| {
                                crate::renderer::hwpunit_to_px(
                                    s.line_height.saturating_add(s.line_spacing),
                                    self.dpi,
                                )
                            })
                            .sum::<f64>()
                    })
                    .unwrap_or(0.0);
                let next_substantial = next_h_px > 30.0;
                let next_doesnt_fit = st.current_height + para_h_px + next_h_px > avail;

                if current_fits && vpos_overflow && next_substantial && next_doesnt_fit {
                    st.advance_column_or_new_page();
                }
            }
        }
    }
}

/// 이 쪽 머리의 저장 vpos — 첫 실 항목의 **이 쪽 첫 줄**이다. 앞 쪽에서 이어진 문단 조각은 원 문단의 첫 줄이 아니라
/// 조각 시작 줄(`start_line`)이고, 앞 쪽에서 이어진 표 조각은 줄 기준 vpos 가 없어 `None`(판정 보류)이다 — 원
/// 문단·표의 첫 줄로 잡으면 쪽 머리가 앞 쪽에 있어 이 쪽 문단이 한 쪽만큼 넘친 것으로 읽힌다(#1659 리뷰 ·
/// 창업도약패키지 채움 3-2-2).
pub(super) fn page_top_stored_vpos(items: &[PageItem], paragraphs: &[Paragraph]) -> Option<i32> {
    items
        .iter()
        .find(|item| !matches!(item, PageItem::EndnoteSeparator { .. }))
        .and_then(|item| match item {
            PageItem::FullParagraph { para_index }
            | PageItem::Table { para_index, .. }
            | PageItem::Shape { para_index, .. } => paragraphs
                .get(*para_index)
                .and_then(|p| p.line_segs.first())
                .map(|s| s.vertical_pos),
            PageItem::PartialParagraph {
                para_index,
                start_line,
                ..
            } => paragraphs
                .get(*para_index)
                .and_then(|p| p.line_segs.get(*start_line))
                .map(|s| s.vertical_pos),
            // 줄 기준 vpos 없음 → 판정 보류.
            PageItem::PartialTable { .. } | PageItem::EndnoteSeparator { .. } => None,
        })
}

/// 다음 문단이 이 쪽에 세워야 하는 첫 조각의 높이(마지막 줄 뒤 간격 제외) — 문단 보호면 통째, 외톨이줄 보호면
/// 두 줄(세 줄 이하는 갈 수 없으니 통째), 보호가 없으면 첫 줄.
fn first_fragment_height(
    fmt: &super::super::paragraph::metrics::FormattedParagraph,
    style: Option<&crate::renderer::style_resolver::ResolvedParaStyle>,
) -> f64 {
    let n = fmt.line_heights.len();
    if n == 0 {
        return 0.0;
    }
    let lines = match style {
        Some(s) if s.keep_lines => n,
        Some(s) if s.widow_orphan && n <= 3 => n,
        Some(s) if s.widow_orphan => 2,
        _ => 1,
    };
    fmt.line_advances_sum(0..lines - 1) + fmt.line_heights[lines - 1]
}
