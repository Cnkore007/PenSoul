// blueprint.rs — 开书定盘蓝图类型定义
// 六张账本：承诺、卷蓝图、人物矩阵、伏笔、副线、资源

use serde::{Deserialize, Serialize};

/// 承诺（对读者的承诺）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub commitment_id: String,
    pub statement: String,
    pub kind: String,
    pub priority: u32,
    pub scope: String,
    pub resolution_chapter: Option<i64>,
    pub ongoing: bool,
    pub status: String,
}

/// 卷蓝图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeBlueprint {
    pub volume_no: u32,
    pub title: String,
    pub one_line: String,
    pub function: String,
    pub chapter_start: i64,
    pub chapter_end: i64,
    pub central_conflict: String,
    pub climax_chapter: Option<i64>,
    pub status: String,
}

/// 人物矩阵条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMatrixEntry {
    pub character_name: String,
    pub role: String,
    pub core_values: Vec<String>,
    pub speech_style: String,
    pub wants: String,
    pub fears: String,
    pub secret: String,
    pub last_appeared: i64,
}

/// 蓝图伏笔
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintForeshadow {
    pub foreshadow_id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub planted_chapter: i64,
    pub expected_payoff_chapter: i64,
    pub status: String,
}

/// 副线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subplot {
    pub subplot_id: String,
    pub name: String,
    pub mainline_relation: String,
    pub status: String,
    pub start_chapter: i64,
    pub end_chapter: Option<i64>,
    pub characters: Vec<String>,
}

/// 资源条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub resource_id: String,
    pub name: String,
    pub rtype: String,
    pub owner: String,
    pub status: String,
}

/// 全书蓝图（六张账本）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookBlueprint {
    pub settled: bool,
    pub settled_at: String,
    pub commitments: Vec<Commitment>,
    pub volumes: Vec<VolumeBlueprint>,
    pub character_matrix: Vec<CharacterMatrixEntry>,
    pub foreshadows: Vec<BlueprintForeshadow>,
    pub subplots: Vec<Subplot>,
    pub resources: Vec<ResourceEntry>,
}
