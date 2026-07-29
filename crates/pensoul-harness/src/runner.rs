/// 执行者矩阵 — 本机/委托执行的正交组合框架。
///
/// 执行者与门控正交组合，覆盖 6 种场景：
/// |              | Auto   | Manual       | Conditional   |
/// |--------------|--------|--------------|---------------|
/// | **Local**    | 批量生成 | 交互式写作    | 自动审查       |
/// | **Delegated**| 自动化流水线 | 专家会诊 | 独立审查      |
///
/// 本模块提供基础框架，具体执行逻辑由上层（pensoul-agent）实现。
use crate::stage::{RunnerType, Stage};
use pensoul_core::StageName;
use std::collections::HashMap;

/// 执行者注册信息。
#[derive(Debug, Clone)]
pub struct RunnerEntry {
    /// 执行者类型。
    pub runner_type: RunnerType,
    /// 执行者描述（面向用户）。
    pub description: String,
    /// 执行者是否就绪。
    pub ready: bool,
}

/// 执行者矩阵，管理所有已注册的执行者。
///
/// 在流程引擎启动时注册可用的执行者，运行时根据阶段的
/// `runner` 字段查找对应的执行者来执行任务。
#[derive(Debug, Clone)]
pub struct RunnerMatrix {
    /// 按阶段名索引的执行者注册表。
    runners: HashMap<StageName, RunnerEntry>,
}

impl RunnerMatrix {
    /// 创建空的执行者矩阵。
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
        }
    }

    /// 注册一个阶段的执行者。
    pub fn register(&mut self, stage_name: StageName, entry: RunnerEntry) {
        self.runners.insert(stage_name, entry);
    }

    /// 获取指定阶段的执行者信息。
    pub fn get(&self, stage_name: &StageName) -> Option<&RunnerEntry> {
        self.runners.get(stage_name)
    }

    /// 检查指定阶段的执行者是否就绪。
    pub fn is_ready(&self, stage_name: &StageName) -> bool {
        self.runners
            .get(stage_name)
            .map(|e| e.ready)
            .unwrap_or(false)
    }

    /// 获取指定阶段的执行者类型。
    pub fn runner_type(&self, stage_name: &StageName) -> Option<&RunnerType> {
        self.runners.get(stage_name).map(|e| &e.runner_type)
    }

    /// 为阶段自动注册默认执行者（本地执行）。
    pub fn register_default(&mut self, stage: &Stage) {
        if !self.runners.contains_key(&stage.name) {
            self.runners.insert(
                stage.name.clone(),
                RunnerEntry {
                    runner_type: stage.runner.clone(),
                    description: format!("默认 {} 执行者", stage.display_name),
                    ready: true,
                },
            );
        }
    }

    /// 已注册的执行者数量。
    pub fn len(&self) -> usize {
        self.runners.len()
    }

    /// 是否没有注册任何执行者。
    pub fn is_empty(&self) -> bool {
        self.runners.is_empty()
    }
}

impl Default for RunnerMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;

    #[test]
    fn test_register_and_get() {
        let mut matrix = RunnerMatrix::new();
        let stage_name = StageName::new("writing");

        matrix.register(
            stage_name.clone(),
            RunnerEntry {
                runner_type: RunnerType::Local,
                description: "本地写作".into(),
                ready: true,
            },
        );

        assert!(matrix.is_ready(&stage_name));
        assert_eq!(matrix.runner_type(&stage_name), Some(&RunnerType::Local));
    }

    #[test]
    fn test_register_default() {
        let mut matrix = RunnerMatrix::new();
        let stage = Stage {
            name: StageName::new("review"),
            runner: RunnerType::Delegated,
            ..Stage::default()
        };

        matrix.register_default(&stage);
        assert_eq!(
            matrix.runner_type(&StageName::new("review")),
            Some(&RunnerType::Delegated)
        );
    }

    #[test]
    fn test_not_registered() {
        let matrix = RunnerMatrix::new();
        assert!(!matrix.is_ready(&StageName::new("unknown")));
        assert!(matrix.runner_type(&StageName::new("unknown")).is_none());
    }
}
