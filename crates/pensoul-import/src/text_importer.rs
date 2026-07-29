use crate::chapter_detect::{ChapterDetector, DetectedChapter};
use pensoul_core::{PensoulError, Result};
/// 文本导入模块
use std::path::Path;

/// 导入结果
#[derive(Debug)]
pub struct ImportResult {
    pub chapters: Vec<DetectedChapter>,
    pub total_words: usize,
    pub file_name: String,
}

/// 文本导入器
pub struct TextImporter;

impl TextImporter {
    /// 导入文本内容
    pub fn import_text(text: &str, min_words: usize) -> ImportResult {
        let detector = ChapterDetector::new();
        let chapters = detector.detect(text, min_words);
        let total_words = chapters.iter().map(|ch| ch.word_count).sum();

        ImportResult {
            chapters,
            total_words,
            file_name: String::new(),
        }
    }

    /// 检查路径安全性（防止目录遍历）
    fn validate_import_path(path: &Path) -> Result<()> {
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(PensoulError::ImportError(
                "路径包含 '..' 目录遍历，已拒绝".to_string(),
            ));
        }
        Ok(())
    }

    /// 从文件导入
    pub fn import_file(path: &str) -> Result<ImportResult> {
        let path = Path::new(path);
        Self::validate_import_path(path)?;
        if !path.exists() {
            return Err(PensoulError::ImportError(format!(
                "文件不存在: {}",
                path.display()
            )));
        }

        let text = std::fs::read_to_string(path)
            .map_err(|e| PensoulError::ImportError(format!("读取文件失败: {}", e)))?;

        let file_name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut result = Self::import_text(&text, 100); // 默认最小字数100
        result.file_name = file_name;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_text_with_chapters() {
        let text = "第1章 标题一\n这是一个足够长的章节内容，包含多个句子。\n第2章 标题二\n这是第二个章节的内容，也需要足够长。";
        let result = TextImporter::import_text(text, 10);
        assert_eq!(result.chapters.len(), 2);
        assert!(result.total_words > 0);
    }

    #[test]
    fn test_import_text_without_chapters() {
        let text = "这是一段没有章节标记的文本，但内容足够长。";
        let result = TextImporter::import_text(text, 10);
        assert_eq!(result.chapters.len(), 1);
        assert_eq!(result.chapters[0].chapter_number, None);
    }

    #[test]
    fn test_import_text_short_content() {
        let text = "短内容";
        let result = TextImporter::import_text(text, 10);
        assert_eq!(result.chapters.len(), 0);
    }

    #[test]
    fn test_import_text_cn_chapters() {
        let text = "第一章 标题一\n内容一内容一内容一内容一内容一内容一内容一内容一内容一内容一。\n第二章 标题二\n内容二内容二内容二内容二内容二内容二内容二内容二内容二内容二。";
        let result = TextImporter::import_text(text, 10);
        assert_eq!(result.chapters.len(), 2);
        assert_eq!(result.chapters[0].chapter_number, Some(1));
        assert_eq!(result.chapters[1].chapter_number, Some(2));
    }

    #[test]
    fn test_import_file_not_found() {
        let result = TextImporter::import_file("nonexistent.txt");
        assert!(result.is_err());
    }
}
