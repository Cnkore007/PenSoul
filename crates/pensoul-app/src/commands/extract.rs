// extract.rs — 事实提取管线（设计第五章 / P1，全自动）
// 章节保存后，由「事实提取」Agent 从正文抽取结构化事实包（Fact），
// 自动写入档案（人物/组织/设定/事件/伏笔），并做冲突检测。
// 安全阀：①硬约束违规告警（full_audit）；②同名新建跳过并告警；
//          ③每次应用写入操作日志（_config/operation-log.json），支持回溯。
// 所有写入真实落盘（正典 + 派生状态重建），不 mock。

use axum::extract::{Form, State};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::agent::{resolve, AgentRole};
use crate::commands::llm::{build_llm_request, llm_client, structured_output_tokens};
use crate::error::ApiError;
use crate::state::AppState;
use pensoul_domain::entity::{Character, Event, ForeshadowStatus, Organization, Setting};
use pensoul_infra::llm::LlmMessage;

#[derive(Deserialize)]
pub struct ExtractParams {
    pub chapter_id: String,
}

/// 单条提取事实（LLM 结构化输出）
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedFact {
    /// character_update / new_character / new_organization / new_event / new_setting / foreshadow_progress
    pub kind: String,
    pub name: String,
    pub attribute: Option<String>,
    pub value: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub chapter: Option<i64>,
    /// 置信度低（指代不明/信息模糊）时标记，供用户抽查
    #[serde(default)]
    pub low_confidence: bool,
}

/// 结构化应用事实（P6 操作日志/回滚用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppliedFact {
    pub kind: String,
    pub name: String,
    pub attribute: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// 提取结果报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtractReport {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub low_confidence: Vec<String>,
}

/// 提取章节事实并自动写入档案（全自动；硬约束冲突阻断告警）
pub async fn extract_facts(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(params): Form<ExtractParams>,
) -> Result<String, ApiError> {
    let (chapter_no, content, base_dir, existing_entities) = {
        let state = state.read().await;
        let ontology = state
            .ontology
            .as_ref()
            .ok_or(ApiError::bad_request("没有打开的项目"))?;
        let chapter = ontology
            .chapters_in_order()
            .into_iter()
            .find(|c| c.chapter_id.to_string() == params.chapter_id)
            .cloned()
            .ok_or(ApiError::not_found("章节不存在"))?;
        if chapter.content.trim().is_empty() {
            return Err(ApiError::bad_request("章节正文为空，无法提取事实"));
        }
        (
            chapter.chapter_no,
            chapter.content.clone(),
            state.base_dir.clone(),
            existing_entities_text(ontology, &chapter.content),
        )
    };

    // 事实提取 Agent（P0b：按角色选模型）
    let provider = resolve(&base_dir, AgentRole::Extractor)?;
    let client = llm_client(&provider);
    let system = extract_system_prompt(&existing_entities);
    // 思考型模型（如 kimi-k3 thinking=Always）会把 reasoning_tokens 计入 max_tokens；
    // 3000 tokens 曾导致正文较长时 JSON 在 closing brace 前被截断，解析失败。
    // 给结构化输出留足“思考 + JSON”预算，同时不越过模型自身最大输出。
    let extract_max_tokens = structured_output_tokens(&provider, 4096, 16000);
    let request = build_llm_request(
        &provider,
        vec![LlmMessage {
            role: "user".to_string(),
            content: truncate_chars(&content, 8000),
        }],
        system,
        true,
        extract_max_tokens,
    );
    let response = client
        .complete(request)
        .await
        .map_err(|e| ApiError::bad_request(format!("事实提取调用失败: {e}")))?;

    // 解析结构化事实：解析失败必须显式报错，禁止静默当"本章无事实"。
    // 兼容两种 LLM 实际输出：提示词要求的 {"facts":[...]} 与部分模型直接返回的 [...]。
    let facts: Vec<ExtractedFact> = parse_extracted_facts(&response.content)
        .map_err(|e| ApiError::bad_request(format!("事实提取响应解析失败: {e}")))?;

    let mut state = state.write().await;
    let (mut report, applied_facts) = apply_facts(&mut state, &params.chapter_id, chapter_no, &facts);
    state.save_project().map_err(ApiError::internal)?;

    // 操作日志（全自动写入的审计轨迹，供回溯/回滚）；写失败不掩盖提取本身，但必须告知用户
    if let Err(e) = append_operation_log(
        &state.base_dir,
        &params.chapter_id,
        &report,
        &applied_facts,
    ) {
        report
            .warnings
            .push(format!("操作日志写入失败（审计轨迹缺失，无法回滚本次变更）: {e}"));
    }

    serde_json::to_string(&report).map_err(|e| ApiError::internal(e.to_string()))
}

/// 应用事实到正典（写路径：正典变更 → 派生重建 → 审计）
fn apply_facts(
    state: &mut AppState,
    _chapter_id: &str,
    chapter_no: i64,
    facts: &[ExtractedFact],
) -> (ExtractReport, Vec<AppliedFact>) {
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    let mut warnings = Vec::new();
    let mut low_confidence = Vec::new();
    let mut applied_facts: Vec<AppliedFact> = Vec::new();

    {
        let ontology = match state.ontology.as_mut() {
            Some(o) => o,
            None => return (ExtractReport { applied, skipped, warnings, low_confidence }, applied_facts),
        };

        for fact in facts {
            let name = fact.name.trim();
            if name.is_empty() {
                continue;
            }
            if fact.low_confidence {
                low_confidence.push(name.to_string());
            }
            match fact.kind.as_str() {
                "character_update" => apply_character_update(ontology, fact, &mut applied, &mut skipped, &mut applied_facts),
                "new_character" => apply_new_character(ontology, fact, &mut applied, &mut skipped, &mut applied_facts),
                "new_organization" => apply_new_organization(ontology, fact, &mut applied, &mut skipped, &mut applied_facts),
                "new_setting" => apply_new_setting(ontology, fact, &mut applied, &mut skipped, &mut applied_facts),
                "new_event" => apply_new_event(ontology, fact, chapter_no, &mut applied, &mut skipped, &mut applied_facts),
                "foreshadow_progress" => apply_foreshadow_progress(ontology, fact, &mut applied, &mut skipped, &mut applied_facts),
                other => skipped.push(format!("未知事实类型 {other}: {name}")),
            }
        }
    }

    // 重建派生状态（图谱/约束/记忆）
    state.rebuild_derived();

    // 硬约束冲突检测（全自动安全阀：只告警阻断，不自动回滚；操作日志可回溯）
    let audit = state.constraints.full_audit();
    for v in audit.violations {
        if v.severity == pensoul_domain::constraint::ViolationSeverity::Error {
            warnings.push(format!("硬约束违规: {}", v.message));
        }
    }

    (ExtractReport { applied, skipped, warnings, low_confidence }, applied_facts)
}

fn apply_character_update(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    let Some(attr) = fact.attribute.as_deref() else {
        skipped.push(format!("角色更新缺少属性: {}", fact.name));
        return;
    };
    let Some(value) = fact.value.as_deref().map(|v| v.trim()).filter(|v| !v.is_empty()) else {
        skipped.push(format!("角色更新缺少值: {}.{}", fact.name, attr));
        return;
    };
    let known_attr = matches!(
        attr,
        "realm"
            | "appearance"
            | "attire"
            | "occupation"
            | "wants"
            | "fears"
            | "secret"
            | "backstory"
            | "techniques"
            | "items"
    );
    if !known_attr {
        skipped.push(format!("未知角色属性 {attr}: {}", fact.name));
        return;
    }

    let existed = ontology
        .characters
        .characters
        .iter()
        .any(|c| c.name == fact.name);
    // 兜底：提取器偶尔会对未建档的主角输出 character_update（模型不知道档案状态）。
    // 这里不静默丢掉事实，而是先补建角色档案，再把属性更新写进去；审计日志保留两笔以便回滚。
    if !existed {
        ontology
            .characters
            .characters
            .push(Character::new(fact.name.clone()));
        applied.push(format!("新角色（由属性更新自动补建）: {}", fact.name));
        applied_facts.push(AppliedFact {
            kind: "new_character".into(),
            name: fact.name.clone(),
            attribute: None,
            old_value: None,
            new_value: None,
        });
    }

    let Some(character) = ontology
        .characters
        .characters
        .iter_mut()
        .find(|c| c.name == fact.name)
    else {
        skipped.push(format!("角色创建失败，无法更新: {}", fact.name));
        return;
    };
    let p = &mut character.properties;
    let old_value = match attr {
        "realm" => p.realm.clone(),
        "appearance" => p.appearance.clone(),
        "attire" => p.attire.clone(),
        "occupation" => p.occupation.clone(),
        "wants" => p.wants.clone(),
        "fears" => p.fears.clone(),
        "secret" => p.secret.clone(),
        "backstory" => p.backstory.clone(),
        // techniques/items 为追加型：old 记空串表示追加
        _ => Some(String::new()),
    };
    let note = match attr {
        "realm" => { p.realm = Some(value.to_string()); "境界" }
        "appearance" => { p.appearance = Some(value.to_string()); "外貌" }
        "attire" => { p.attire = Some(value.to_string()); "衣着" }
        "occupation" => { p.occupation = Some(value.to_string()); "职业" }
        "wants" => { p.wants = Some(value.to_string()); "欲望" }
        "fears" => { p.fears = Some(value.to_string()); "恐惧" }
        "secret" => { p.secret = Some(value.to_string()); "秘密" }
        "backstory" => { p.backstory = Some(value.to_string()); "背景" }
        "techniques" => {
            for t in value.split(['，', ',']) {
                let t = t.trim();
                if !t.is_empty() && !p.techniques.iter().any(|x| x == t) {
                    p.techniques.push(t.to_string());
                }
            }
            "功法"
        }
        "items" => {
            for it in value.split(['，', ',']) {
                let it = it.trim();
                if !it.is_empty() && !p.items.iter().any(|x| x == it) {
                    p.items.push(it.to_string());
                }
            }
            "法宝"
        }
        _ => unreachable!("known_attr 已白名单校验"),
    };
    let update_label = if existed {
        format!("角色 {}: {} → {}", fact.name, note, value)
    } else {
        format!("新角色 {}: {} → {}", fact.name, note, value)
    };
    applied.push(update_label);
    applied_facts.push(AppliedFact {
        kind: "character_update".into(),
        name: fact.name.clone(),
        attribute: Some(attr.to_string()),
        old_value,
        new_value: Some(value.to_string()),
    });
}

fn apply_new_character(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    if ontology.characters.characters.iter().any(|c| c.name == fact.name) {
        skipped.push(format!("角色已存在，跳过新建: {}", fact.name));
        return;
    }
    let mut character = Character::new(fact.name.clone());
    if let Some(occupation) = fact.description.as_deref().filter(|s| !s.is_empty()) {
        character.properties.occupation = Some(occupation.to_string());
    }
    if let Some(category) = fact.category.as_deref().filter(|s| !s.is_empty()) {
        // 类别即职业/身份描述
        character.properties.occupation = Some(category.to_string());
    }
    ontology.characters.characters.push(character);
    applied.push(format!("新角色: {}", fact.name));
    applied_facts.push(AppliedFact {
        kind: "new_character".into(),
        name: fact.name.clone(),
        attribute: None,
        old_value: None,
        new_value: None,
    });
}

fn apply_new_organization(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    if ontology.world.organizations.iter().any(|o| o.name == fact.name) {
        skipped.push(format!("组织已存在，跳过新建: {}", fact.name));
        return;
    }
    let category = fact.category.clone().unwrap_or_else(|| "势力".to_string());
    let mut org = Organization::new(fact.name.clone(), category);
    org.description = fact.description.clone().unwrap_or_default();
    ontology.world.organizations.push(org);
    applied.push(format!("新组织: {}", fact.name));
    applied_facts.push(AppliedFact {
        kind: "new_organization".into(),
        name: fact.name.clone(),
        attribute: None,
        old_value: None,
        new_value: None,
    });
}

fn apply_new_setting(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    if ontology.world.locations.iter().any(|s| s.name == fact.name) {
        skipped.push(format!("设定已存在，跳过新建: {}", fact.name));
        return;
    }
    let category = fact.category.clone().unwrap_or_else(|| "地点".to_string());
    let mut setting = Setting::new(fact.name.clone(), category);
    setting.description = fact.description.clone().unwrap_or_default();
    ontology.world.locations.push(setting);
    applied.push(format!("新设定: {}", fact.name));
    applied_facts.push(AppliedFact {
        kind: "new_setting".into(),
        name: fact.name.clone(),
        attribute: None,
        old_value: None,
        new_value: None,
    });
}

fn apply_new_event(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    chapter_no: i64,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    let chapter = fact.chapter.unwrap_or(chapter_no).max(1);
    if ontology.world.timeline.iter().any(|e| e.name == fact.name && e.chapter_id == chapter) {
        skipped.push(format!("事件已存在，跳过新建: {}", fact.name));
        return;
    }
    let mut event = Event::new(fact.name.clone(), chapter);
    event.description = fact.description.clone().unwrap_or_default();
    ontology.world.timeline.push(event);
    applied.push(format!("新事件（第{chapter}章）: {}", fact.name));
    applied_facts.push(AppliedFact {
        kind: "new_event".into(),
        name: fact.name.clone(),
        attribute: None,
        old_value: None,
        new_value: None,
    });
}

fn apply_foreshadow_progress(
    ontology: &mut pensoul_domain::ontology::NovelOntology,
    fact: &ExtractedFact,
    applied: &mut Vec<String>,
    skipped: &mut Vec<String>,
    applied_facts: &mut Vec<AppliedFact>,
) {
    let Some(foreshadow) = ontology.narrative.foreshadows.iter_mut().find(|f| f.name == fact.name) else {
        skipped.push(format!("伏笔不存在，无法推进: {}", fact.name));
        return;
    };
    // 状态值白名单：未知值显式跳过并记录，禁止静默映射为 Progressing
    let next = match fact.value.as_deref().unwrap_or("") {
        "Planted" => ForeshadowStatus::Planted,
        "Progressing" => ForeshadowStatus::Progressing,
        "Resolved" => ForeshadowStatus::Resolved,
        other => {
            skipped.push(format!(
                "伏笔状态值非法（{other:?}），跳过: {}",
                fact.name
            ));
            return;
        }
    };
    // 状态机门控：非法跳转由约束审计发现（此处仍写入，审计告警）
    let old_status = format!("{:?}", foreshadow.status);
    let new_status = format!("{next:?}");
    foreshadow.status = next;
    applied.push(format!("伏笔推进: {}", fact.name));
    applied_facts.push(AppliedFact {
        kind: "foreshadow_progress".into(),
        name: fact.name.clone(),
        attribute: Some("status".into()),
        old_value: Some(old_status),
        new_value: Some(new_status),
    });
}


/// 解析事实提取的结构化输出。
///
/// 提示词要求输出 `{"facts":[...]}`，但不同模型/网关可能直接返回 `[...]`。
/// 先走统一的宽松 JSON 解析（容忍 Markdown 代码块与前后说明文字），
/// 再按对象/数组两种形态归一化为 `Vec<ExtractedFact>`。
fn parse_extracted_facts(raw: &str) -> Result<Vec<ExtractedFact>, String> {
    let value: serde_json::Value = pensoul_infra::llm::parse_llm_json(raw)?;
    let facts_value = if let Some(array) = value.get("facts").and_then(|v| v.as_array()) {
        array.clone()
    } else if let Some(array) = value.as_array() {
        array.clone()
    } else {
        return Err(format!(
            "LLM 输出缺少 facts 数组；原始内容前 300 字: {}",
            raw.trim().chars().take(300).collect::<String>()
        ));
    };
    serde_json::from_value(serde_json::Value::Array(facts_value)).map_err(|e| e.to_string())
}

/// 给提取器一份「当前档案中与本章正文相关的实体清单」，只用于判断 update / new。
/// 按名称在正文中出现做过滤，避免 500 万字级作品把所有历史实体塞进每章提示词；
/// 未建档主角自然不在清单中，会被模型输出为 new_character。
fn existing_entities_text(
    ontology: &pensoul_domain::ontology::NovelOntology,
    chapter_text: &str,
) -> String {
    let mentioned = |name: &str| !name.is_empty() && chapter_text.contains(name);
    serde_json::json!({
        "characters": ontology
            .characters
            .characters
            .iter()
            .map(|c| c.name.as_str())
            .filter(|name| mentioned(name))
            .collect::<Vec<_>>(),
        "organizations": ontology
            .world
            .organizations
            .iter()
            .map(|o| o.name.as_str())
            .filter(|name| mentioned(name))
            .collect::<Vec<_>>(),
        "settings": ontology
            .world
            .locations
            .iter()
            .map(|s| s.name.as_str())
            .filter(|name| mentioned(name))
            .collect::<Vec<_>>(),
        "foreshadows": ontology
            .narrative
            .foreshadows
            .iter()
            .map(|f| f.name.as_str())
            .filter(|name| mentioned(name))
            .collect::<Vec<_>>(),
    })
    .to_string()
}

fn extract_system_prompt(existing_entities: &str) -> String {
    let mut prompt = String::from(
        "你是 PenSoul 的事实提取器：从章节正文中抽取「档案事实」，只输出 JSON，不要任何解释。\n\
         {\n\
           \"facts\": [\n\
             {\"kind\": \"character_update\", \"name\": \"角色名\", \"attribute\": \"realm|appearance|attire|occupation|wants|fears|secret|backstory|techniques|items\", \"value\": \"新值\", \"low_confidence\": false},\n\
             {\"kind\": \"new_character\", \"name\": \"角色名\", \"category\": \"身份/职业\"},\n\
             {\"kind\": \"new_organization\", \"name\": \"组织名\", \"category\": \"宗门|家族|帝国|商会\", \"description\": \"简述\"},\n\
             {\"kind\": \"new_setting\", \"name\": \"设定名\", \"category\": \"地点|体系|功法|法宝|境界\", \"description\": \"规则或描述\"},\n\
             {\"kind\": \"new_event\", \"name\": \"事件名\", \"description\": \"简述\"},\n\
             {\"kind\": \"foreshadow_progress\", \"name\": \"伏笔名\", \"value\": \"Planted|Progressing|Resolved\"}\n\
           ]\n\
         }\n",
    );
    prompt.push_str("当前档案已有实体（JSON，只用于判断应使用 update 还是 new）：\n");
    prompt.push_str(existing_entities);
    prompt.push_str(
        "\n\
         规则：\n\
         0. 正文角色已在「characters」清单中才可用 character_update；不在清单中的角色一律用 new_character，即使他是主角；\n\
         1. 只提取正文中明确出现的事实；指代不明、猜测内容标记 low_confidence=true；\n\
         2. 组织/设定同理：已存在才考虑属性变化，新登场实体用 new_*；\n\
         3. 功法、法宝等可追加多个，用 value 传单个新增项；\n\
         4. 不要编造正文中没有的信息；空结果返回 {\"facts\":[]}。",
    );
    prompt
}

/// 追加操作日志（全自动写入的审计轨迹；facts 为结构化变更，供回滚）
/// 写失败必须返回错误，由调用方把缺失审计轨迹的警告带给用户。
fn append_operation_log(
    base_dir: &str,
    chapter_id: &str,
    report: &ExtractReport,
    applied_facts: &[AppliedFact],
) -> Result<(), String> {
    let dir = std::path::Path::new(base_dir).join("_config");
    let path = dir.join("operation-log.json");
    std::fs::create_dir_all(&dir).map_err(|e| format!("日志目录创建失败: {e}"))?;
    let mut entries: Vec<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    entries.push(serde_json::json!({
        "time": chrono::Utc::now().to_rfc3339(),
        "chapter_id": chapter_id,
        "applied": report.applied,
        "skipped": report.skipped,
        "warnings": report.warnings,
        "facts": applied_facts,
    }));
    // 保留最近 500 条，防止无限膨胀
    if entries.len() > 500 {
        entries.drain(..entries.len() - 500);
    }
    let body = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("操作日志写入失败: {e}"))
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut output: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        output.push_str("…（正文过长已截断）");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn parse_extracted_facts_accepts_prompt_object_shape() {
        let raw = r#"```json
{"facts":[{"kind":"new_character","name":"监工","category":"血奴营监工"}]}
```"#;
        let facts = parse_extracted_facts(raw).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].kind, "new_character");
        assert_eq!(facts[0].name, "监工");
    }

    #[test]
    fn parse_extracted_facts_accepts_bare_array() {
        let facts = parse_extracted_facts(
            r#"[{"kind":"new_setting","name":"血池","category":"地点","description":"血奴营核心"}]"#,
        )
        .unwrap();
        assert_eq!(facts[0].name, "血池");
    }

    #[test]
    fn parse_extracted_facts_accepts_empty_result() {
        let facts = parse_extracted_facts(r#"{"facts":[]}"#).unwrap();
        assert!(facts.is_empty());
    }

    #[test]
    fn parse_extracted_facts_rejects_missing_facts_array() {
        let err = parse_extracted_facts(r#"{"unexpected":true}"#).unwrap_err();
        assert!(err.contains("缺少 facts 数组"));
    }

    #[test]
    fn extract_prompt_is_stable() {
        let prompt = extract_system_prompt("{\"characters\":[\"赵梓\"]}");
        assert!(prompt.contains("character_update"));
        assert!(prompt.contains("new_character"));
        assert!(prompt.contains("low_confidence"));
        assert!(prompt.contains("赵梓"));
    }

    #[test]
    fn apply_facts_creates_and_updates() {
        let base_dir = tempfile::TempDir::new().unwrap();
        let mut state = AppState::new(base_dir.path().to_string_lossy().to_string());
        let mut ontology = pensoul_domain::ontology::NovelOntology::new(
            pensoul_domain::id::ProjectId::default(),
            "测试",
        );
        ontology.characters.characters.push(Character::new("林默"));
        state.ontology = Some(ontology);
        state.rebuild_derived();

        let facts = vec![
            ExtractedFact {
                kind: "new_organization".into(),
                name: "青云宗".into(),
                attribute: None,
                value: None,
                category: Some("宗门".into()),
                description: Some("执掌正道".into()),
                chapter: None,
                low_confidence: false,
            },
            ExtractedFact {
                kind: "character_update".into(),
                name: "林默".into(),
                attribute: Some("realm".into()),
                value: Some("筑基后期".into()),
                category: None,
                description: None,
                chapter: None,
                low_confidence: false,
            },
            ExtractedFact {
                kind: "character_update".into(),
                name: "林默".into(),
                attribute: Some("techniques".into()),
                value: Some("青云剑诀,御风术".into()),
                category: None,
                description: None,
                chapter: None,
                low_confidence: false,
            },
        ];
        let (report, applied_facts) = apply_facts(&mut state, "ch1", 3, &facts);
        assert!(report.applied.iter().any(|a| a.contains("青云宗")));
        assert!(report.applied.iter().any(|a| a.contains("筑基后期")));
        assert!(report.applied.iter().any(|a| a.contains("功法")));
        assert!(applied_facts.iter().any(|f| f.kind == "new_organization"), "应记录结构化事实");

        let ontology = state.ontology.unwrap();
        assert_eq!(ontology.world.organizations.len(), 1);
        let hero = ontology.characters.characters.iter().find(|c| c.name == "林默").unwrap();
        assert_eq!(hero.properties.realm.as_deref(), Some("筑基后期"));
        assert_eq!(hero.properties.techniques.len(), 2);
    }

    #[test]
    fn apply_facts_autocreates_missing_character_from_update() {
        let base_dir = tempfile::TempDir::new().unwrap();
        let mut state = AppState::new(base_dir.path().to_string_lossy().to_string());
        let mut ontology = pensoul_domain::ontology::NovelOntology::new(
            pensoul_domain::id::ProjectId::default(),
            "测试",
        );
        ontology.characters.characters.push(Character::new("林默"));
        state.ontology = Some(ontology);
        state.rebuild_derived();

        let facts = vec![
            ExtractedFact { kind: "new_character".into(), name: "林默".into(), attribute: None, value: None, category: None, description: None, chapter: None, low_confidence: false },
            ExtractedFact { kind: "character_update".into(), name: "不存在的人".into(), attribute: Some("realm".into()), value: Some("炼气".into()), category: None, description: None, chapter: None, low_confidence: false },
        ];
        let (report, applied_facts) = apply_facts(&mut state, "ch1", 1, &facts);
        assert!(report.skipped.iter().any(|s| s.contains("已存在")));
        assert!(
            report.applied.iter().any(|s| s.contains("不存在的人")),
            "未建档角色收到 character_update 时应自动补建: {:?}",
            report.applied
        );
        assert!(applied_facts.iter().any(|f| f.kind == "new_character" && f.name == "不存在的人"));
        assert!(applied_facts.iter().any(|f| f.kind == "character_update" && f.name == "不存在的人"));

        let ontology = state.ontology.unwrap();
        let created = ontology
            .characters
            .characters
            .iter()
            .find(|c| c.name == "不存在的人")
            .unwrap();
        assert_eq!(created.properties.realm.as_deref(), Some("炼气"));
    }
}
