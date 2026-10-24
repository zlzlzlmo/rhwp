//! 중첩 표(셀 안에 든 표) 구조 쓰기 — 셀 경로판.
//!
//! 평면 API(`insert_table_row_native` …)는 `(구역, 문단, 컨트롤)` 로 본문 **최상위** 표만 닿는다.
//! 칸 안에 든 표는 그 좌표계로 지칭할 수 없어, 셀 경로(`[(컨트롤, 셀, 셀 문단), …]`, 마지막
//! 항목이 대상 표)를 받는 쌍둥이가 필요하다. 읽기(`getTableDimensionsByPath` …)와 칸 크기
//! (`resizeTableCellsByPath`)는 이미 경로를 받는다 — 여기는 줄·칸 넣기/지우기, 칸 합치기/나누기,
//! 표/셀 속성이다.
//!
//! 규약은 `resize_table_cells_by_cell_path_native` 와 같다: 깊이 1 경로는 평면 API 에 그대로
//! 위임하고(같은 결과), 깊이 ≥ 2 는 경로 끝 표를 `resolve_table_mut_by_cell_path` 로 찾아 같은
//! `Table` 메서드를 부른 뒤 **안쪽 칸 폭 기준**으로 글줄을 다시 짠다.

use super::super::helpers::json_ok_with;
use crate::document_core::DocumentCore;
use crate::error::HwpError;
use crate::model::event::DocumentEvent;

/// 경로 끝 표의 셀 `cell_idx` 의 문단 `cell_para_idx` 를 가리키는 경로 — 끝 항목만 바꾼다.
fn cell_para_path(
    path: &[(usize, usize, usize)],
    cell_idx: usize,
    cell_para_idx: usize,
) -> Vec<(usize, usize, usize)> {
    let mut cell_path = path.to_vec();
    if let Some(last) = cell_path.last_mut() {
        *last = (last.0, cell_idx, cell_para_idx);
    }
    cell_path
}

/// 경로의 첫 컨트롤 번호 — 이벤트가 가리키는 본문 최상위 표다. 빈 경로는 거절한다.
fn outermost_control(path: &[(usize, usize, usize)]) -> Result<usize, HwpError> {
    path.first()
        .map(|&(control_idx, _, _)| control_idx)
        .ok_or_else(|| HwpError::RenderError("경로가 비어있습니다".to_string()))
}

impl DocumentCore {
    /// 표 구조가 바뀐 뒤 구역을 다시 짜고 쪽을 나눈다 — 평면 API 와 같은 마무리다.
    fn finish_nested_table_edit(&mut self, section_idx: usize) {
        self.document.sections[section_idx].raw_stream = None;
        self.recompose_section(section_idx);
        self.paginate_if_needed();
    }

    /// 경로 끝 표의 셀 `cell_idx` 문단 `cell_para_idx` 를 **그 셀 폭 기준**으로 다시 짠다.
    /// 깊이 1 은 평면 `reflow_cell_paragraph` 와 같은 호출이다.
    pub(crate) fn reflow_table_cell_paragraph_at(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        cell_idx: usize,
        cell_para_idx: usize,
    ) {
        if let [(control_idx, _, _)] = path {
            self.reflow_cell_paragraph(
                section_idx,
                parent_para_idx,
                *control_idx,
                cell_idx,
                cell_para_idx,
            );
            return;
        }
        let cell_path = cell_para_path(path, cell_idx, cell_para_idx);
        self.reflow_cell_paragraph_by_path(section_idx, parent_para_idx, &cell_path, cell_para_idx);
    }

    /// 중첩 표에 줄을 넣는다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    pub fn insert_table_row_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        row_idx: u16,
        below: bool,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.insert_table_row_native(
                section_idx,
                parent_para_idx,
                control_idx,
                row_idx,
                below,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table
            .insert_row(row_idx, below)
            .map_err(HwpError::RenderError)?;
        let (row_count, col_count) = (table.row_count, table.col_count);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::TableRowInserted {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!(
            "\"rowCount\":{},\"colCount\":{}",
            row_count, col_count
        )))
    }

    /// 중첩 표에 칸을 넣는다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    pub fn insert_table_column_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        col_idx: u16,
        right: bool,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.insert_table_column_native(
                section_idx,
                parent_para_idx,
                control_idx,
                col_idx,
                right,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table
            .insert_column(col_idx, right)
            .map_err(HwpError::RenderError)?;
        let (row_count, col_count) = (table.row_count, table.col_count);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::TableColumnInserted {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!(
            "\"rowCount\":{},\"colCount\":{}",
            row_count, col_count
        )))
    }

    /// 중첩 표의 줄을 지운다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    pub fn delete_table_row_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        row_idx: u16,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.delete_table_row_native(
                section_idx,
                parent_para_idx,
                control_idx,
                row_idx,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table.delete_row(row_idx).map_err(HwpError::RenderError)?;
        let (row_count, col_count) = (table.row_count, table.col_count);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::TableRowDeleted {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!(
            "\"rowCount\":{},\"colCount\":{}",
            row_count, col_count
        )))
    }

    /// 중첩 표의 칸을 지운다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    pub fn delete_table_column_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        col_idx: u16,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.delete_table_column_native(
                section_idx,
                parent_para_idx,
                control_idx,
                col_idx,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table
            .delete_column(col_idx)
            .map_err(HwpError::RenderError)?;
        let (row_count, col_count) = (table.row_count, table.col_count);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::TableColumnDeleted {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!(
            "\"rowCount\":{},\"colCount\":{}",
            row_count, col_count
        )))
    }

    /// 중첩 표의 셀을 합친다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    ///
    /// 합친 주 셀의 폭이 흡수한 셀만큼 넓어지므로 그 셀의 문단을 **안쪽 셀 폭**으로 다시 짠다.
    pub fn merge_table_cells_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        start_row: u16,
        start_col: u16,
        end_row: u16,
        end_col: u16,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.merge_table_cells_native(
                section_idx,
                parent_para_idx,
                control_idx,
                start_row,
                start_col,
                end_row,
                end_col,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table
            .merge_cells(start_row, start_col, end_row, end_col)
            .map_err(HwpError::RenderError)?;
        let cell_count = table.cells.len();
        // 병합 뒤 주 셀은 (row, col) 순으로 다시 정렬되므로 인덱스가 아니라 좌표로 다시 찾는다.
        let merged = table
            .cells
            .iter()
            .position(|c| c.row == start_row && c.col == start_col)
            .map(|idx| (idx, table.cells[idx].paragraphs.len()));
        if let Some((cell_idx, para_count)) = merged {
            for cell_para_idx in 0..para_count {
                self.reflow_table_cell_paragraph_at(
                    section_idx,
                    parent_para_idx,
                    path,
                    cell_idx,
                    cell_para_idx,
                );
            }
        }
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::CellsMerged {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!("\"cellCount\":{}", cell_count)))
    }

    /// 중첩 표의 합쳐진 셀을 나눈다 (셀 경로). 깊이 1 은 평면 API 에 위임한다.
    pub fn split_table_cell_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        row: u16,
        col: u16,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.split_table_cell_native(
                section_idx,
                parent_para_idx,
                control_idx,
                row,
                col,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table.split_cell(row, col).map_err(HwpError::RenderError)?;
        let cell_count = table.cells.len();
        self.reflow_stale_cells_after_split_at(section_idx, parent_para_idx, path);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::CellSplit {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!("\"cellCount\":{}", cell_count)))
    }

    /// 칸 나누기 뒤 폭이 좁아진 중첩 표 셀의 stale 글줄을 다시 짜고 vpos 사다리를 처음부터 세운다 —
    /// 평면 `reflow_stale_cells_after_split` 의 경로판이다(대상 판별은 `stale_split_cells` 한 벌).
    fn reflow_stale_cells_after_split_at(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
    ) {
        let stale_cells = match self.resolve_table_by_path(section_idx, parent_para_idx, path) {
            Ok(table) => Self::stale_split_cells(table, &Self::table_cell_reflow_metrics(table)),
            Err(_) => return,
        };
        for &(cell_idx, para_count) in &stale_cells {
            for cell_para_idx in 0..para_count {
                let cell_path = cell_para_path(path, cell_idx, cell_para_idx);
                self.reflow_cell_paragraph_by_path_after_split(
                    section_idx,
                    parent_para_idx,
                    &cell_path,
                    cell_para_idx,
                );
            }
        }
        for &(cell_idx, _) in &stale_cells {
            let cell_path = cell_para_path(path, cell_idx, 0);
            self.rebuild_cell_vpos_ladder_by_path(section_idx, parent_para_idx, &cell_path);
        }
    }

    /// 중첩 표의 한 칸을 N줄 × M칸으로 나눈다 (셀 경로).
    pub fn split_table_cell_into_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        row: u16,
        col: u16,
        n_rows: u16,
        m_cols: u16,
        equal_row_height: bool,
        merge_first: bool,
    ) -> Result<String, HwpError> {
        let control_idx = outermost_control(path)?;
        if path.len() == 1 {
            return self.split_table_cell_into_native(
                section_idx,
                parent_para_idx,
                control_idx,
                row,
                col,
                n_rows,
                m_cols,
                equal_row_height,
                merge_first,
            );
        }
        let table = self.resolve_table_mut_by_cell_path(section_idx, parent_para_idx, path)?;
        table
            .split_cell_into(row, col, n_rows, m_cols, equal_row_height, merge_first)
            .map_err(HwpError::RenderError)?;
        let cell_count = table.cells.len();
        self.reflow_stale_cells_after_split_at(section_idx, parent_para_idx, path);
        self.finish_nested_table_edit(section_idx);
        self.event_log.push(DocumentEvent::CellSplit {
            section: section_idx,
            para: parent_para_idx,
            ctrl: control_idx,
        });
        Ok(json_ok_with(&format!("\"cellCount\":{}", cell_count)))
    }

    /// 중첩 표의 셀 속성을 고친다 (셀 경로). 본문은 평면 API 와 한 벌(`set_cell_properties_at_native`)이다.
    pub fn set_cell_properties_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        cell_idx: usize,
        json: &str,
    ) -> Result<String, HwpError> {
        outermost_control(path)?;
        self.set_cell_properties_at_native(section_idx, parent_para_idx, path, cell_idx, json)
    }

    /// 중첩 표의 표 속성을 읽는다 (셀 경로).
    pub fn get_table_properties_by_cell_path_native(
        &self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
    ) -> Result<String, HwpError> {
        outermost_control(path)?;
        self.get_table_properties_at_native(section_idx, parent_para_idx, path)
    }

    /// 중첩 표의 표 속성을 고친다 (셀 경로). 본문은 평면 API 와 한 벌(`set_table_properties_at_native`)이다.
    pub fn set_table_properties_by_cell_path_native(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        path: &[(usize, usize, usize)],
        json: &str,
    ) -> Result<String, HwpError> {
        outermost_control(path)?;
        self.set_table_properties_at_native(section_idx, parent_para_idx, path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::control::Control;
    use crate::model::document::{Document, Section};
    use crate::model::paragraph::Paragraph;
    use crate::model::table::{Cell, Table, VerticalAlign};

    const OUTER_CELL_WIDTH: u32 = 30000;
    const INNER_CELL_WIDTH: u32 = 5000;
    const INNER_CELL_HEIGHT: u32 = 2000;

    fn para(text: &str) -> Paragraph {
        Paragraph {
            text: text.to_string(),
            char_offsets: (0..text.chars().count() as u32).collect(),
            char_count: text.chars().count() as u32,
            has_para_text: true,
            ..Default::default()
        }
    }

    /// `rows × cols` 표 — 칸 글은 `r{행}c{열}`.
    fn grid_table(rows: u16, cols: u16) -> Table {
        let mut cells = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                cells.push(Cell {
                    row,
                    col,
                    col_span: 1,
                    row_span: 1,
                    width: INNER_CELL_WIDTH,
                    height: INNER_CELL_HEIGHT,
                    paragraphs: vec![para(&format!("r{row}c{col}"))],
                    ..Default::default()
                });
            }
        }
        let mut table = Table {
            row_count: rows,
            col_count: cols,
            cells,
            ..Default::default()
        };
        table.rebuild_grid();
        table.update_ctrl_dimensions();
        table
    }

    /// 바깥 1×1 표의 칸 안에 `rows × cols` 표가 든 문서. 반환 경로는 `hitTest` 의 `cellPath` 모양이다.
    fn nested_core(rows: u16, cols: u16) -> (DocumentCore, Vec<(usize, usize, usize)>) {
        let mut outer_cell_para = Paragraph::default();
        outer_cell_para
            .controls
            .push(Control::Table(Box::new(grid_table(rows, cols))));
        let mut outer = Table {
            row_count: 1,
            col_count: 1,
            cells: vec![Cell {
                row: 0,
                col: 0,
                col_span: 1,
                row_span: 1,
                width: OUTER_CELL_WIDTH,
                height: 9000,
                paragraphs: vec![outer_cell_para],
                ..Default::default()
            }],
            ..Default::default()
        };
        outer.rebuild_grid();
        outer.update_ctrl_dimensions();
        let mut body = Paragraph::default();
        body.controls.push(Control::Table(Box::new(outer)));
        (core_with(body), vec![(0, 0, 0), (0, 0, 0)])
    }

    /// 본문 최상위에 `rows × cols` 표 하나만 든 문서(평면 API 대조용).
    fn flat_core(rows: u16, cols: u16) -> DocumentCore {
        let mut body = Paragraph::default();
        body.controls
            .push(Control::Table(Box::new(grid_table(rows, cols))));
        core_with(body)
    }

    fn core_with(body: Paragraph) -> DocumentCore {
        let mut document = Document::default();
        document.sections.push(Section {
            paragraphs: vec![body],
            ..Default::default()
        });
        let mut core = DocumentCore::new_empty();
        core.set_document(document);
        core
    }

    fn outer_table(core: &DocumentCore) -> &Table {
        match core.document().sections[0].paragraphs[0].controls.first() {
            Some(Control::Table(table)) => table,
            _ => panic!("바깥 표를 찾지 못했다"),
        }
    }

    fn inner_table(core: &DocumentCore) -> &Table {
        match outer_table(core).cells[0].paragraphs[0].controls.first() {
            Some(Control::Table(table)) => table,
            _ => panic!("안쪽 표를 찾지 못했다"),
        }
    }

    fn flat_table(core: &DocumentCore) -> &Table {
        outer_table(core)
    }

    /// 행 우선으로 늘어놓은 칸 글.
    fn texts(table: &Table) -> Vec<String> {
        table
            .cells
            .iter()
            .map(|cell| cell.paragraphs[0].text.clone())
            .collect()
    }

    /// 바깥 표가 손대이지 않았음을 한 번에 잰다.
    fn assert_outer_untouched(core: &DocumentCore) {
        let outer = outer_table(core);
        assert_eq!(outer.row_count, 1, "바깥 표의 줄 수는 그대로");
        assert_eq!(outer.col_count, 1, "바깥 표의 칸 수는 그대로");
        assert_eq!(outer.cells.len(), 1, "바깥 표의 셀 수는 그대로");
        assert_eq!(outer.cells[0].width, OUTER_CELL_WIDTH);
    }

    // ─── 줄·칸 넣기/지우기 ──────────────────────────────────────────

    #[test]
    fn insert_row_below_grows_the_inner_table_only() {
        let (mut core, path) = nested_core(2, 2);
        core.insert_table_row_by_cell_path_native(0, 0, &path, 0, true)
            .expect("중첩 표 줄 넣기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 3);
        assert_eq!(inner.col_count, 2);
        assert_eq!(inner.cells.len(), 6);
        // 0행 아래에 넣었으니 새 줄은 1행, 옛 1행은 2행으로 밀린다.
        assert_eq!(
            inner
                .cells
                .iter()
                .filter(|cell| cell.row == 2)
                .map(|cell| cell.paragraphs[0].text.as_str())
                .collect::<Vec<_>>(),
            vec!["r1c0", "r1c1"]
        );
        assert_outer_untouched(&core);
    }

    #[test]
    fn insert_row_above_shifts_the_target_row_down() {
        let (mut core, path) = nested_core(2, 2);
        core.insert_table_row_by_cell_path_native(0, 0, &path, 0, false)
            .expect("중첩 표 줄 넣기(위)가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 3);
        assert_eq!(
            inner
                .cells
                .iter()
                .filter(|cell| cell.row == 1)
                .map(|cell| cell.paragraphs[0].text.as_str())
                .collect::<Vec<_>>(),
            vec!["r0c0", "r0c1"],
            "0행 위에 넣으면 옛 0행이 1행으로 밀린다"
        );
        assert_outer_untouched(&core);
    }

    #[test]
    fn insert_column_right_grows_the_inner_table_only() {
        let (mut core, path) = nested_core(2, 2);
        core.insert_table_column_by_cell_path_native(0, 0, &path, 0, true)
            .expect("중첩 표 칸 넣기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 2);
        assert_eq!(inner.col_count, 3);
        assert_eq!(inner.cells.len(), 6);
        assert_eq!(
            inner
                .cells
                .iter()
                .filter(|cell| cell.col == 2)
                .map(|cell| cell.paragraphs[0].text.as_str())
                .collect::<Vec<_>>(),
            vec!["r0c1", "r1c1"],
            "0열 오른쪽에 넣으면 옛 1열이 2열로 밀린다"
        );
        assert_outer_untouched(&core);
    }

    #[test]
    fn delete_row_removes_the_inner_row_only() {
        let (mut core, path) = nested_core(2, 2);
        core.delete_table_row_by_cell_path_native(0, 0, &path, 1)
            .expect("중첩 표 줄 지우기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 1);
        assert_eq!(texts(inner), vec!["r0c0", "r0c1"]);
        assert_outer_untouched(&core);
    }

    #[test]
    fn delete_column_removes_the_inner_column_only() {
        let (mut core, path) = nested_core(2, 2);
        core.delete_table_column_by_cell_path_native(0, 0, &path, 0)
            .expect("중첩 표 칸 지우기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.col_count, 1);
        assert_eq!(texts(inner), vec!["r0c1", "r1c1"]);
        assert_outer_untouched(&core);
    }

    // ─── 칸 합치기/나누기 ───────────────────────────────────────────

    #[test]
    fn merge_cells_merges_inside_the_inner_table_only() {
        let (mut core, path) = nested_core(2, 2);
        core.merge_table_cells_by_cell_path_native(0, 0, &path, 0, 0, 0, 1)
            .expect("중첩 표 칸 합치기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.cells.len(), 3, "넷에서 둘이 하나가 되어 셋");
        assert_eq!(inner.cells[0].col_span, 2);
        assert_eq!(
            inner.cells[0].width,
            INNER_CELL_WIDTH * 2,
            "합친 칸의 폭은 흡수한 칸만큼 넓어진다"
        );
        assert_outer_untouched(&core);
    }

    #[test]
    fn split_cell_restores_the_merged_inner_cell() {
        let (mut core, path) = nested_core(2, 2);
        core.merge_table_cells_by_cell_path_native(0, 0, &path, 0, 0, 0, 1)
            .expect("합치기");
        core.split_table_cell_by_cell_path_native(0, 0, &path, 0, 0)
            .expect("중첩 표 칸 나누기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.cells.len(), 4);
        assert!(inner.cells.iter().all(|cell| cell.col_span == 1));
        assert_outer_untouched(&core);
    }

    // ─── 속성 ──────────────────────────────────────────────────────

    #[test]
    fn set_cell_properties_changes_the_inner_cell_only() {
        let (mut core, path) = nested_core(2, 2);
        let outer_padding_before = outer_table(&core).cells[0].padding.left;

        core.set_cell_properties_by_cell_path_native(
            0,
            0,
            &path,
            1,
            r#"{"paddingLeft":333,"verticalAlign":1}"#,
        )
        .expect("중첩 표 셀 속성이 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.cells[1].padding.left, 333);
        assert_eq!(inner.cells[1].vertical_align, VerticalAlign::Center);
        assert_ne!(
            inner.cells[0].padding.left, 333,
            "다른 안쪽 칸은 건드리지 않는다"
        );
        assert_eq!(
            outer_table(&core).cells[0].padding.left,
            outer_padding_before,
            "바깥 칸은 건드리지 않는다"
        );
    }

    #[test]
    fn set_cell_properties_border_change_stays_inside_the_inner_table() {
        let (mut core, path) = nested_core(2, 2);
        let outer_bf_before = outer_table(&core).cells[0].border_fill_id;
        let inner_bf_before = inner_table(&core).cells[0].border_fill_id;

        core.set_cell_properties_by_cell_path_native(
            0,
            0,
            &path,
            0,
            r##"{"borderLeft":{"type":1,"width":2,"color":"#ff0000"},
                "borderRight":{"type":1,"width":2,"color":"#ff0000"},
                "borderTop":{"type":1,"width":2,"color":"#ff0000"},
                "borderBottom":{"type":1,"width":2,"color":"#ff0000"},
                "fillType":"none"}"##,
        )
        .expect("중첩 표 셀 테두리가 성공해야 한다");

        assert_ne!(
            inner_table(&core).cells[0].border_fill_id,
            inner_bf_before,
            "안쪽 칸의 테두리 서식이 바뀐다"
        );
        assert_eq!(
            outer_table(&core).cells[0].border_fill_id,
            outer_bf_before,
            "바깥 칸의 테두리 서식은 그대로"
        );
    }

    #[test]
    fn set_table_properties_changes_the_inner_table_only() {
        let (mut core, path) = nested_core(2, 2);
        let outer_spacing_before = outer_table(&core).cell_spacing;

        core.set_table_properties_by_cell_path_native(
            0,
            0,
            &path,
            r#"{"cellSpacing":40,"paddingTop":250}"#,
        )
        .expect("중첩 표 표 속성이 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.cell_spacing, 40);
        assert_eq!(inner.padding.top, 250);
        assert_eq!(outer_table(&core).cell_spacing, outer_spacing_before);
    }

    #[test]
    fn set_table_properties_border_change_stays_inside_the_inner_table() {
        let (mut core, path) = nested_core(2, 2);
        let outer_bf_before = outer_table(&core).cells[0].border_fill_id;

        core.set_table_properties_by_cell_path_native(
            0,
            0,
            &path,
            r##"{"borderLeft":{"type":1,"width":2,"color":"#0000ff"},
                "borderRight":{"type":1,"width":2,"color":"#0000ff"},
                "borderTop":{"type":1,"width":2,"color":"#0000ff"},
                "borderBottom":{"type":1,"width":2,"color":"#0000ff"},
                "fillType":"none"}"##,
        )
        .expect("중첩 표 테두리가 성공해야 한다");

        let inner = inner_table(&core);
        let new_bf = inner.border_fill_id;
        assert!(inner.cells.iter().all(|cell| cell.border_fill_id == new_bf));
        assert_eq!(outer_table(&core).cells[0].border_fill_id, outer_bf_before);
    }

    // ─── 규약 ──────────────────────────────────────────────────────

    #[test]
    fn depth_one_path_matches_the_flat_api() {
        let mut by_path = flat_core(2, 2);
        let mut flat = flat_core(2, 2);
        let path = vec![(0usize, 0usize, 0usize)];

        by_path
            .insert_table_row_by_cell_path_native(0, 0, &path, 0, true)
            .expect("경로판");
        flat.insert_table_row_native(0, 0, 0, 0, true)
            .expect("평면");
        assert_eq!(
            texts(flat_table(&by_path)),
            texts(flat_table(&flat)),
            "깊이 1 경로는 평면 API 와 같은 결과다"
        );

        by_path
            .merge_table_cells_by_cell_path_native(0, 0, &path, 0, 0, 0, 1)
            .expect("경로판");
        flat.merge_table_cells_native(0, 0, 0, 0, 0, 0, 1)
            .expect("평면");
        assert_eq!(
            flat_table(&by_path).cells.len(),
            flat_table(&flat).cells.len()
        );

        by_path
            .set_table_properties_by_cell_path_native(0, 0, &path, r#"{"cellSpacing":12}"#)
            .expect("경로판");
        flat.set_table_properties_native(0, 0, 0, r#"{"cellSpacing":12}"#)
            .expect("평면");
        assert_eq!(
            flat_table(&by_path).cell_spacing,
            flat_table(&flat).cell_spacing
        );
    }

    #[test]
    fn bad_paths_fail_and_leave_the_document_alone() {
        let (mut core, path) = nested_core(2, 2);

        assert!(
            core.insert_table_row_by_cell_path_native(0, 0, &[], 0, true)
                .is_err(),
            "빈 경로는 거절한다"
        );
        // 안쪽 컨트롤 번호가 범위 밖
        let mut bad = path.clone();
        bad[1].0 = 7;
        assert!(core
            .delete_table_row_by_cell_path_native(0, 0, &bad, 0)
            .is_err());
        // 구역·문단 번호가 범위 밖
        assert!(core
            .delete_table_row_by_cell_path_native(3, 0, &path, 0)
            .is_err());
        assert!(core
            .delete_table_row_by_cell_path_native(0, 9, &path, 0)
            .is_err());
        // 안쪽 줄 번호가 범위 밖 — 표는 그대로
        assert!(core
            .delete_table_row_by_cell_path_native(0, 0, &path, 5)
            .is_err());
        assert!(core
            .set_cell_properties_by_cell_path_native(0, 0, &path, 9, r#"{"paddingLeft":1}"#)
            .is_err());

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 2);
        assert_eq!(texts(inner), vec!["r0c0", "r0c1", "r1c0", "r1c1"]);
        assert_outer_untouched(&core);
    }

    /// `get_table_bbox_native` 가 돌려주는 JSON 에서 표 높이(px)를 읽는다.
    fn table_height_px(core: &DocumentCore) -> f64 {
        let json = core
            .get_table_bbox_native(0, 0, 0)
            .expect("바깥 표의 상자를 잴 수 있어야 한다");
        let rest = json.split("\"height\":").nth(1).expect("height 키");
        rest.trim_end_matches('}')
            .parse::<f64>()
            .expect("height 는 숫자")
    }

    #[test]
    fn outer_table_follows_the_inner_table_when_rows_are_added() {
        let (mut core, path) = nested_core(2, 2);
        core.paginate();
        let before = table_height_px(&core);

        for _ in 0..6 {
            core.insert_table_row_by_cell_path_native(0, 0, &path, 0, true)
                .expect("중첩 표 줄 넣기");
        }
        core.paginate();
        let after = table_height_px(&core);

        assert!(
            after > before,
            "안쪽 표가 자라면 바깥 표가 따라 자라야 한다 (겹치지 않는다): {before} → {after}"
        );
    }

    #[test]
    fn split_cell_into_splits_one_inner_cell_into_a_grid() {
        let (mut core, path) = nested_core(1, 1);
        core.split_table_cell_into_by_cell_path_native(0, 0, &path, 0, 0, 2, 2, true, false)
            .expect("중첩 표 칸 N×M 나누기가 성공해야 한다");

        let inner = inner_table(&core);
        assert_eq!(inner.row_count, 2);
        assert_eq!(inner.col_count, 2);
        assert_eq!(inner.cells.len(), 4);
        assert_outer_untouched(&core);

        // 깊이 1 경로는 평면 API 와 같은 결과다.
        let mut by_path = flat_core(1, 1);
        let mut flat = flat_core(1, 1);
        by_path
            .split_table_cell_into_by_cell_path_native(0, 0, &[(0, 0, 0)], 0, 0, 2, 3, true, false)
            .expect("경로판");
        flat.split_table_cell_into_native(0, 0, 0, 0, 0, 2, 3, true, false)
            .expect("평면");
        assert_eq!(
            flat_table(&by_path).cells.len(),
            flat_table(&flat).cells.len()
        );
        assert_eq!(flat_table(&by_path).col_count, flat_table(&flat).col_count);
    }

    #[test]
    fn get_table_properties_reads_the_inner_table() {
        let (mut core, path) = nested_core(2, 2);
        core.set_table_properties_by_cell_path_native(0, 0, &path, r#"{"cellSpacing":40}"#)
            .expect("속성 쓰기");

        let inner = core
            .get_table_properties_by_cell_path_native(0, 0, &path)
            .expect("중첩 표 속성 읽기가 성공해야 한다");
        assert!(
            inner.contains("\"cellSpacing\":40"),
            "안쪽 표의 값: {inner}"
        );
        let outer = core
            .get_table_properties_native(0, 0, 0)
            .expect("바깥 표 속성 읽기");
        assert!(
            outer.contains("\"cellSpacing\":0"),
            "바깥 표의 값은 그대로: {outer}"
        );

        // 깊이 1 경로는 평면 API 와 같은 값이다.
        let flat = flat_core(2, 2);
        assert_eq!(
            flat.get_table_properties_by_cell_path_native(0, 0, &[(0, 0, 0)])
                .expect("깊이 1"),
            flat.get_table_properties_native(0, 0, 0).expect("평면"),
        );
        assert!(
            core.get_table_properties_by_cell_path_native(0, 0, &[])
                .is_err(),
            "빈 경로는 거절한다"
        );
    }

    /// 바깥 1×1 → 가운데 `rows × cols` → 그 (0,0) 칸 안의 `inner_rows × inner_cols` 표. 3단 경로 `[(0,0,0), (0,0,0), (0,0,0)]`.
    fn deep_core(
        rows: u16,
        cols: u16,
        inner_rows: u16,
        inner_cols: u16,
    ) -> (DocumentCore, Vec<(usize, usize, usize)>) {
        let mut middle = grid_table(rows, cols);
        let mut cell_para = Paragraph::default();
        cell_para
            .controls
            .push(Control::Table(Box::new(grid_table(inner_rows, inner_cols))));
        middle.cells[0].paragraphs = vec![cell_para];
        let mut outer_cell_para = Paragraph::default();
        outer_cell_para
            .controls
            .push(Control::Table(Box::new(middle)));
        let mut outer = Table {
            row_count: 1,
            col_count: 1,
            cells: vec![Cell {
                row: 0,
                col: 0,
                col_span: 1,
                row_span: 1,
                width: OUTER_CELL_WIDTH,
                height: 9000,
                paragraphs: vec![outer_cell_para],
                ..Default::default()
            }],
            ..Default::default()
        };
        outer.rebuild_grid();
        outer.update_ctrl_dimensions();
        let mut body = Paragraph::default();
        body.controls.push(Control::Table(Box::new(outer)));
        (core_with(body), vec![(0, 0, 0), (0, 0, 0), (0, 0, 0)])
    }

    fn deepest_table(core: &DocumentCore) -> &Table {
        let Some(Control::Table(middle)) =
            outer_table(core).cells[0].paragraphs[0].controls.first()
        else {
            panic!("가운데 표를 찾지 못했다");
        };
        match middle.cells[0].paragraphs[0].controls.first() {
            Some(Control::Table(table)) => table,
            _ => panic!("가장 안쪽 표를 찾지 못했다"),
        }
    }

    #[test]
    fn three_level_path_edits_only_the_deepest_table() {
        let (mut core, path) = deep_core(2, 2, 2, 2);
        core.insert_table_row_by_cell_path_native(0, 0, &path, 0, true)
            .expect("3단 경로 줄 넣기");
        core.merge_table_cells_by_cell_path_native(0, 0, &path, 0, 0, 0, 1)
            .expect("3단 경로 칸 합치기");
        core.set_table_properties_by_cell_path_native(0, 0, &path, r#"{"cellSpacing":55}"#)
            .expect("3단 경로 표 속성");

        let deepest = deepest_table(&core);
        assert_eq!(deepest.row_count, 3);
        assert_eq!(deepest.cells.len(), 5, "6칸에서 둘이 하나로 합쳐 다섯");
        assert_eq!(deepest.cell_spacing, 55);
        let Some(Control::Table(middle)) =
            outer_table(&core).cells[0].paragraphs[0].controls.first()
        else {
            panic!("가운데 표");
        };
        assert_eq!(
            (middle.row_count, middle.col_count, middle.cells.len()),
            (2, 2, 4),
            "가운데 표는 그대로"
        );
        assert_eq!(middle.cell_spacing, 0);
        assert_outer_untouched(&core);
        // 읽기도 3단 경로로 같은 표를 본다.
        assert!(core
            .get_table_properties_by_cell_path_native(0, 0, &path)
            .expect("3단 경로 읽기")
            .contains("\"cellSpacing\":55"));
    }

    #[test]
    fn last_row_of_an_inner_table_cannot_be_deleted() {
        let (mut core, path) = nested_core(1, 2);
        // 평면 API 와 같은 규칙 — 마지막 한 줄은 지울 수 없다.
        let mut flat = flat_core(1, 2);
        assert!(flat.delete_table_row_native(0, 0, 0, 0).is_err());
        assert!(core
            .delete_table_row_by_cell_path_native(0, 0, &path, 0)
            .is_err());
        assert_eq!(inner_table(&core).row_count, 1);
    }
}
