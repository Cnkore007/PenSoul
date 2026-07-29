/// Write-Ahead Log — 阶段流转的完整审计轨迹。
///
/// 每一次状态变更都先写入 WAL，再执行实际操作。
/// 崩溃恢复时通过 WAL 重放来还原引擎状态。
///
/// WAL 条目使用 blake3 校验和防篡改。
use pensoul_core::{PensoulError, Result};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

// ── WAL 动作 ──────────────────────────────────────────────────────────────

/// WAL 记录的动作类型。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WalAction {
    /// 引擎初始化。
    EngineInit,
    /// 滚动备忘录注入。
    MemoInject,
    /// 阶段开始。
    StageStart,
    /// 阶段完成。
    StageComplete,
    /// 门控通过。
    GatePass,
    /// 门控未通过。
    GateFail,
    /// 推进到下一阶段。
    Advance,
    /// 工具访问被拦截。
    ToolBlocked,
    /// 整个 Harness 流程完成。
    HarnessComplete,
    /// 状态同步（快照持久化）。
    StateSync,
}

// ── WAL 条目 ──────────────────────────────────────────────────────────────

/// 单条 WAL 记录。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalEntry {
    /// 时间戳（Unix 秒，浮点）。
    pub timestamp: f64,
    /// 动作类型。
    pub action: WalAction,
    /// 关联的阶段名称（可选）。
    pub stage: Option<String>,
    /// 附加数据。
    pub data: Option<String>,
    /// blake3 校验和（hex 编码）。
    pub checksum: String,
}

impl WalEntry {
    /// 创建新条目并自动计算校验和。
    pub fn new(action: WalAction, stage: Option<String>, data: Option<String>) -> Self {
        let timestamp = now_timestamp();
        let checksum = Self::compute_checksum_static(timestamp, &action, &stage, &data);
        Self {
            timestamp,
            action,
            stage,
            data,
            checksum,
        }
    }

    /// 计算条目的 blake3 校验和。
    ///
    /// 输入 = timestamp + action + stage + data，避免序列化开销。
    fn compute_checksum_static(
        timestamp: f64,
        action: &WalAction,
        stage: &Option<String>,
        data: &Option<String>,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(timestamp.to_string().as_bytes());
        hasher.update(format!("{action:?}").as_bytes());
        if let Some(s) = stage {
            hasher.update(s.as_bytes());
        }
        if let Some(d) = data {
            hasher.update(d.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    /// 验证本条目的校验和是否正确。
    pub fn verify_checksum(&self) -> bool {
        let expected =
            Self::compute_checksum_static(self.timestamp, &self.action, &self.stage, &self.data);
        self.checksum == expected
    }
}

// ── WAL 管理器 ────────────────────────────────────────────────────────────

/// WAL 管理器，负责条目的写入、刷盘、校验和状态持久化。
#[derive(Debug, Clone)]
pub struct WalManager {
    /// WAL 文件路径。
    wal_path: PathBuf,
    /// 引擎状态快照文件路径。
    state_path: PathBuf,
}

impl WalManager {
    /// 创建新的 WAL 管理器。
    ///
    /// 文件结构：
    /// - `{project_dir}/.harness/wal.log` — WAL 条目
    /// - `{project_dir}/.harness/state.json` — 引擎状态快照
    pub fn new(project_dir: &Path) -> Self {
        let harness_dir = project_dir.join(".harness");
        fs::create_dir_all(&harness_dir).ok();

        let wal_path = harness_dir.join("wal.log");
        let state_path = harness_dir.join("state.json");

        Self {
            wal_path,
            state_path,
        }
    }

    /// 写入一条 WAL 条目并刷盘。
    ///
    /// # 参数
    /// - `action`: 动作类型。
    /// - `stage`: 关联阶段名称（可选）。
    /// - `data`: 附加数据（可选）。
    pub fn write(&self, action: WalAction, stage: Option<&str>, data: Option<&str>) -> Result<()> {
        let entry = WalEntry::new(action, stage.map(String::from), data.map(String::from));

        // 追加写入文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.wal_path)
            .map_err(|e| PensoulError::IoError(format!("打开 WAL 文件失败: {e}")))?;

        let line = serde_json::to_string(&entry)
            .map_err(|e| PensoulError::SerializationError(format!("序列化 WAL 条目失败: {e}")))?;
        writeln!(file, "{line}")
            .map_err(|e| PensoulError::IoError(format!("写入 WAL 文件失败: {e}")))?;

        // 刷盘
        file.flush()
            .map_err(|e| PensoulError::IoError(format!("刷新 WAL 文件失败: {e}")))?;

        Ok(())
    }

    /// 写入一条 WAL 条目并刷盘（可变引用版本，同时追加到内存）。
    pub fn write_mut(
        &mut self,
        action: WalAction,
        stage: Option<&str>,
        data: Option<&str>,
    ) -> Result<()> {
        let entry = WalEntry::new(action, stage.map(String::from), data.map(String::from));

        // 追加写入文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.wal_path)
            .map_err(|e| PensoulError::IoError(format!("打开 WAL 文件失败: {e}")))?;

        let line = serde_json::to_string(&entry)
            .map_err(|e| PensoulError::SerializationError(format!("序列化 WAL 条目失败: {e}")))?;
        writeln!(file, "{line}")
            .map_err(|e| PensoulError::IoError(format!("写入 WAL 文件失败: {e}")))?;

        file.flush()
            .map_err(|e| PensoulError::IoError(format!("刷新 WAL 文件失败: {e}")))?;

        Ok(())
    }

    /// 强制刷盘。
    pub fn flush(&self) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.wal_path)
            .map_err(|e| PensoulError::IoError(format!("打开 WAL 文件失败: {e}")))?;
        file.sync_all()
            .map_err(|e| PensoulError::IoError(format!("刷新 WAL 文件失败: {e}")))?;
        Ok(())
    }

    /// 保存引擎状态快照到磁盘。
    pub fn save_state(&self, state: &serde_json::Value) -> Result<()> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| PensoulError::SerializationError(format!("序列化状态失败: {e}")))?;
        fs::write(&self.state_path, json)
            .map_err(|e| PensoulError::IoError(format!("写入状态文件失败: {e}")))?;
        Ok(())
    }

    /// 从磁盘加载 WAL 条目。
    pub fn load_entries(&self) -> Result<Vec<WalEntry>> {
        self.load_entries_from_file()
    }

    /// 校验所有条目的 checksum 完整性。
    ///
    /// 返回第一个校验失败的条目索引；全部通过返回 `Ok(())`。
    pub fn verify_integrity(entries: &[WalEntry]) -> Result<()> {
        for (i, entry) in entries.iter().enumerate() {
            if !entry.verify_checksum() {
                return Err(PensoulError::WalChecksumFailed { index: i });
            }
        }
        Ok(())
    }

    /// 获取 WAL 文件路径。
    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    /// 获取状态文件路径。
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    // ── 内部方法 ────────────────────────────────────────────────────────

    /// 从文件读取 WAL 条目。
    fn load_entries_from_file(&self) -> Result<Vec<WalEntry>> {
        if !self.wal_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.wal_path)
            .map_err(|e| PensoulError::IoError(format!("打开 WAL 文件失败: {e}")))?;

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line_result in reader.lines() {
            let line =
                line_result.map_err(|e| PensoulError::IoError(format!("读取 WAL 行失败: {e}")))?;
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            let entry: WalEntry = serde_json::from_str(&line).map_err(|e| {
                PensoulError::SerializationError(format!("反序列化 WAL 条目失败: {e}"))
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }
}

/// 获取当前 Unix 时间戳（秒，浮点数）。
fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_write_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let mut wal = WalManager::new(tmp.path());

        wal.write_mut(WalAction::EngineInit, None, Some("test init"))
            .unwrap();
        wal.write_mut(WalAction::StageStart, Some("writing"), None)
            .unwrap();

        let loaded = wal.load_entries().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].action, WalAction::EngineInit);
        assert_eq!(loaded[1].stage.as_deref(), Some("writing"));
    }

    #[test]
    fn test_wal_checksum_integrity() {
        let tmp = tempfile::tempdir().unwrap();
        let mut wal = WalManager::new(tmp.path());

        wal.write_mut(WalAction::EngineInit, None, None).unwrap();
        wal.write_mut(WalAction::StageStart, Some("w"), None)
            .unwrap();

        let entries = wal.load_entries().unwrap();
        assert!(WalManager::verify_integrity(&entries).is_ok());
    }

    #[test]
    fn test_wal_checksum_tampered() {
        let mut entry = WalEntry::new(WalAction::EngineInit, None, None);
        let good_checksum = entry.checksum.clone();
        entry.checksum = "tampered".to_string();

        // 手动构造一个损坏的条目来测试 verify_checksum
        assert!(!entry.verify_checksum());
        // 恢复后应该通过
        entry.checksum = good_checksum;
        assert!(entry.verify_checksum());
    }

    #[test]
    fn test_wal_save_and_load_state() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = WalManager::new(tmp.path());

        let state = serde_json::json!({"current_stage": "writing", "attempt": 3});
        wal.save_state(&state).unwrap();

        let loaded = fs::read_to_string(wal.state_path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&loaded).unwrap();
        assert_eq!(parsed["current_stage"], "writing");
    }

    #[test]
    fn test_wal_empty_file_load() {
        let tmp = tempfile::tempdir().unwrap();
        let wal = WalManager::new(tmp.path());
        let entries = wal.load_entries().unwrap();
        assert!(entries.is_empty());
    }
}
