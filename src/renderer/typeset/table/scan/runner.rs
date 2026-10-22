//! 표 도메인 조정. 기존 평가·상태 쓰기 순서를 보존한다.

use crate::renderer::typeset::{
    rowbreak_table_has_internal_saved_vpos_reset, table, BlockRowScanVars, BlockTableRowScan,
    MeasuredTable, ResolvedStyleSet, TypesetEngine, TypesetState,
};

#[derive(Clone, Copy)]
struct ScanInput<'a> {
    st: &'a TypesetState,
    layout_engine: &'a crate::renderer::layout::LayoutEngine,
    mt: &'a MeasuredTable,
    table: &'a crate::model::table::Table,
    styles: &'a ResolvedStyleSet,
    cut_row_h: &'a [f64],
    whole_row_fit_h: &'a [f64],
    rowspan_touched: &'a [bool],
    start_cut: &'a [usize],
    v: BlockRowScanVars,
    table_storage_declares_splits: bool,
}

#[derive(Clone)]
struct ScanProgress {
    r: usize,
    scan: BlockTableRowScan,
    bleed_absorbed_row_height: Option<f64>,
}

struct ScanStep {
    progress: ScanProgress,
    keep_scanning: bool,
}

mod block_step;
mod row_step;

impl TypesetEngine {
    /// [Task #2085] 표 분할점 행-스캔: cursor_row 부터 이번 조각(가용 높이
    /// avail_for_rows)에 들어가는 행/블록/셀-컷을 결정한다. 원본 무변경 통이동 —
    /// landscape/rowbreak 허용치의 소스분기(is_hwpx_source)는 caller 에 잔류.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::renderer::typeset) fn scan_block_table_split_rows(
        &self,
        st: &TypesetState,
        layout_engine: &crate::renderer::layout::LayoutEngine,
        mt: &MeasuredTable,
        table: &crate::model::table::Table,
        styles: &ResolvedStyleSet,
        cut_row_h: &[f64],
        whole_row_fit_h: &[f64],
        rowspan_touched: &[bool],
        start_cut: &[usize],
        v: BlockRowScanVars,
        scan: BlockTableRowScan,
    ) -> BlockTableRowScan {
        let table_storage_declares_splits = rowbreak_table_has_internal_saved_vpos_reset(table);
        let input = ScanInput {
            st,
            layout_engine,
            mt,
            table,
            styles,
            cut_row_h,
            whole_row_fit_h,
            rowspan_touched,
            start_cut,
            v,
            table_storage_declares_splits,
        };
        let mut progress = ScanProgress {
            r: v.cursor_row,
            scan,
            bleed_absorbed_row_height: None,
        };
        while progress.r < v.row_count {
            let r = progress.r;
            let cs_before = if r > v.cursor_row { v.cs } else { 0.0 };
            // Consume a carried physical tail before scanning the next row.
            if r == v.cursor_row {
                if let Some(height) = v.start_row_height_override {
                    progress.scan.consumed += height;
                    progress.r += 1;
                    progress.scan.end_row = progress.r;
                    continue;
                }
            }
            let block_query = table::scan::RowBlockQuery {
                layout_engine,
                mt,
                table,
                styles,
                cut_row_h,
                rowspan_touched,
                cs: v.cs,
                row_limit: st.base_available_height() * 0.4,
            };
            let block = block_query.candidate(r);
            // 한글(맥 12.30)은 RowBreak 표 중간(조각 첫 행이 아닌)의 «선언 빈 띠» 행을 rowspan 이 걸려 있어도 행 안에서 가른다 —
            // 초창패 채움본 2쪽 8×5 표 6행(이미지 칸): 빈 칸 158.9px 행을 쪽 끝에서 띠째 가르고 «이미지» 라벨은 그 조각 높이의 가운데에 선다.
            // 기존 블록 단계가 묶음을 놓았거나 눌러 넣은 경우(1741000 squeeze)는 그대로 두고, **한 행도 못 놓고 묶음째 미루려 할 때만**
            // 일반 행 단계로 다시 시도한다. 표 첫 블록(#1145 · 조각 첫 행)은 대상이 아니다 — 맥도 제목 rowspan 첫 블록은 다음 쪽으로 넘긴다.
            let block_step_applies = (block.protected || block.rowbreak_rowspan_block) && block.b_start == r;
            let band_fallback_snapshot = (block_step_applies
                && mt.allows_row_break_split()
                && r > v.cursor_row
                && table::scan::row::row_is_declared_empty_band(mt, table, r))
            .then(|| progress.clone());
            let mut step = if block_step_applies {
                self.scan_rowspan_block_step(input, progress, block, cs_before)
            } else {
                self.scan_ordinary_row_step(input, progress, cs_before)
            };
            if let Some(snapshot) = band_fallback_snapshot {
                if step.progress.r == r && step.progress.scan.end_row == snapshot.scan.end_row {
                    step = self.scan_ordinary_row_step(input, snapshot, cs_before);
                }
            }
            progress = step.progress;
            if !step.keep_scanning {
                break;
            }
        }
        progress.scan
    }
}
