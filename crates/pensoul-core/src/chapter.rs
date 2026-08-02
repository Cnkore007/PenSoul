/// 章节结构类型定义
use crate::id::{ChapterId, VolumeId};

/// 行内批注的锚点：定位到段落 + 段内偏移 + 锚定原文片段
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AnnotationAnchor {
    /// 正文段落索引（按 \n\n 分段，从 0 起）
    pub paragraph_index: usize,
    /// 段内字符偏移
    pub offset: usize,
    /// 锚定原文片段（保存时校验/重新定位用）
    pub text: String,
    /// 字段级锚点（细纲/描述等表单字段名）；行内批注为 None
    #[serde(default)]
    pub field: Option<String>,
}

/// 全链路批注（正文行内 / 表单字段 / 实体级）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChapterAnnotation {
    pub annotation_id: String,
    /// issue=问题 / suggestion=修改建议 / note=备注
    pub kind: String,
    /// 行内批注有锚点；整章批注为 None
    pub anchor: Option<AnnotationAnchor>,
    /// 批注内容（含修改建议）
    pub content: String,
    /// open=待处理 / accepted=已采纳 / rejected=已拒绝
    #[serde(default)]
    pub status: String,
    /// 创建时间
    #[serde(default)]
    pub created_at: String,
    /// 处理该批注的章节版本（0 = 未处理）
    #[serde(default)]
    pub processed_in_version: i32,
    /// 定位串：如 chapter:ch-1:body / location:loc-1:description
    #[serde(default)]
    pub target: Option<String>,
    /// 判决来源：manual=用户直接处理 / rewrite_plan=重写计划（LLM 提案+用户默许）
    #[serde(default)]
    pub resolved_by: Option<String>,
    /// 批注创建时的锚定文本快照（漂移检测与学习数据用）
    #[serde(default)]
    pub anchor_snapshot: Option<String>,
    /// 处理时间
    #[serde(default)]
    pub resolved_at: Option<String>,
}

/// 章节版本历史（批注重写前快照 / 回滚点）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChapterRevision {
    pub version: i32,
    pub content: String,
    #[serde(default)]
    pub word_count: u32,
    #[serde(default)]
    pub created_at: String,
    /// 生成原因：批注重写前快照 / 回滚 / 初始
    #[serde(default)]
    pub reason: String,
}

/// 章节
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chapter {
    /// 章节 ID
    pub chapter_id: ChapterId,
    /// 章节序号（全书第几章，从 1 开始；0 表示未分配，加载时按数组顺序回填）
    /// 记忆管道/影响图/一致性等顺序语义一律用本字段，不依赖 chapter_id 可解析为数字
    #[serde(default)]
    pub chapter_no: i64,
    /// 卷 ID
    pub volume_id: VolumeId,
    /// 章节标题
    pub title: String,
    /// 章节梗概（大纲层信息，非正文）
    #[serde(default)]
    pub summary: String,
    /// 章节内容
    pub content: String,
    /// 字数
    pub word_count: u32,
    /// 版本号
    pub version: i32,
    /// 章节状态
    pub status: ChapterStatus,
    /// 一致性分数
    pub consistency_score: f32,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 笔耕批注（行内 + 整章），旧项目 JSON 无此字段
    #[serde(default)]
    pub annotations: Vec<ChapterAnnotation>,
    /// 版本历史（可回滚），旧项目 JSON 无此字段
    #[serde(default)]
    pub revisions: Vec<ChapterRevision>,
}

/// 章节状态
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChapterStatus {
    /// 草稿
    Draft,
    /// 审阅中
    Reviewing,
    /// 已审阅
    Reviewed,
    /// 已润色
    Polished,
    /// 已发布
    Published,
}

/// 卷
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Volume {
    /// 卷 ID
    pub volume_id: VolumeId,
    /// 卷标题
    pub title: String,
    /// 章节 ID 列表
    pub chapter_ids: Vec<ChapterId>,
    /// 卷摘要
    pub summary: String,
    /// 大纲页是否展开该卷（前端视图状态，持久化避免切页回弹）
    #[serde(default = "default_true")]
    pub expanded: bool,
}

fn default_true() -> bool {
    true
}
