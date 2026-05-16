//! Tabular extractor for CSV / TSV.
//!
//! Table files are indexed at file granularity. The extractor stays registered
//! so the pipeline captures file-level source, but it deliberately emits no
//! per-column leaves.

use super::FileExtractor;
use super::common::ExtractionResult;
use super::language::{FileKind, TableFormat};

pub struct TableExtractor {
    format: TableFormat,
}

impl TableExtractor {
    pub fn new(format: TableFormat) -> Self {
        Self { format }
    }
}

impl FileExtractor for TableExtractor {
    fn file_kind(&self) -> FileKind {
        FileKind::Table(self.format)
    }

    fn extract(&self, _source: &str) -> ExtractionResult {
        ExtractionResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_files_emit_no_column_leaves() {
        let src = "id,name,email\n1,alice,a@x\n2,bob,b@x\n";
        let out = TableExtractor::new(TableFormat::Csv).extract(src);
        assert!(out.leaves.is_empty());
    }

    #[test]
    fn tsv_files_emit_no_column_leaves() {
        let src = "id\tname\temail\n1\talice\ta@x\n";
        let out = TableExtractor::new(TableFormat::Tsv).extract(src);
        assert!(out.leaves.is_empty());
    }

    #[test]
    fn empty_input_yields_zero_leaves() {
        let out = TableExtractor::new(TableFormat::Csv).extract("");
        assert!(out.leaves.is_empty());
    }

    #[test]
    fn whitespace_header_still_produces_no_leaves() {
        let src = "id, name ,,email\n";
        let out = TableExtractor::new(TableFormat::Csv).extract(src);
        assert!(out.leaves.is_empty());
    }
}
