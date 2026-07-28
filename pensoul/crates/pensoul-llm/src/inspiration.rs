/// 灵感生成模块
///
/// 根据创作上下文类型生成创意建议。
/// 优先使用 LLM 生成，LLM 不可用时返回精选的灵感卡片。
use crate::model::ModelConfig;
use crate::provider::LlmProvider;

/// 灵感建议
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InspirationItem {
    /// 建议标题
    pub title: String,
    /// 建议详细内容
    pub content: String,
}

/// 生成灵感建议
///
/// - `provider` — LLM 提供商（可选，为 None 则仅返回精选内容）
/// - `model` — 模型配置
/// - `context_type` — 创作上下文类型
/// - `context_data` — 当前项目的上下文数据（JSON 格式）
///   - context_data 最大长度限制（字符数）
const MAX_CONTEXT_DATA_LEN: usize = 10000;

pub fn generate_inspiration(
    provider: Option<&dyn LlmProvider>,
    model: Option<&ModelConfig>,
    context_type: &str,
    context_data: &str,
) -> Vec<InspirationItem> {
    // 限制 context_data 长度，防止 LLM token 溢出
    let context_data = if context_data.len() > MAX_CONTEXT_DATA_LEN {
        &context_data[..MAX_CONTEXT_DATA_LEN]
    } else {
        context_data
    };

    // 1. 尝试 LLM 生成
    if let (Some(prov), Some(mdl)) = (provider, model) {
        let prompt = build_prompt(context_type, context_data);
        match prov.call(mdl, &prompt) {
            Ok(text) => {
                let parsed = parse_llm_response(&text);
                if !parsed.is_empty() {
                    return parsed;
                }
            }
            Err(e) => {
                eprintln!("[灵感] LLM 生成失败，回退到精选内容: {e}");
            }
        }
    }

    // 2. 回退到精选灵感卡片
    curated_inspiration(context_type)
}

/// 构建 LLM prompt
fn build_prompt(context_type: &str, context_data: &str) -> String {
    match context_type {
        "character" => format!(
            r#"你是一位资深的小说角色创作顾问。

当前项目的角色设定数据如下：
{}

请提供 5 条关于角色创作的灵感建议，每条建议针对不同方面（如角色背景、性格冲突、成长弧光、关系张力、动机设计等）。
要求：
- 每条建议以 "【建议】" 开头
- 每条建议 2-3 句话，具体而不空洞
- 如果有既有角色，建议应结合现有设定
- 如果没有角色，给出从零创建角色的实用思路

请严格按以下格式输出，每条一行：

【建议】<建议内容>
【建议】<建议内容>
..."#,
            context_data
        ),
        "world" => format!(
            r#"你是一位资深的世界观构建顾问。

当前项目的世界观设定数据如下：
{}

请提供 5 条关于世界观构建的灵感建议，涵盖地点设计、时间线发展、设定规则等方面。
要求：
- 每条建议以 "【建议】" 开头
- 每条建议 2-3 句话，具体而不空洞
- 结合现有设定给出延伸方向
- 如果没有设定，给出从零构建的实用思路

请严格按以下格式输出，每条一行：

【建议】<建议内容>
【建议】<建议内容>
..."#,
            context_data
        ),
        "outline" => format!(
            r#"你是一位资深的故事结构顾问。

当前项目的大纲数据如下：
{}

请提供 5 条关于故事结构和大纲的灵感建议，涵盖情节设计、悬念设置、节奏控制、人物弧光、主题深化等方面。
要求：
- 每条建议以 "【建议】" 开头
- 每条建议 2-3 句话，具体而不空洞
- 结合已有大纲给出延伸方向
- 如果没有大纲，给出从零搭建故事骨架的实用思路

请严格按以下格式输出，每条一行：

【建议】<建议内容>
【建议】<建议内容>
..."#,
            context_data
        ),
        "writing" => format!(
            r#"你是一位资深的小说写作顾问。

当前正在创作的章节上下文如下：
{}

请提供 5 条关于正文写作的灵感建议，涵盖叙事视角、场景描写、对话设计、节奏变化、情感渲染等方面。
要求：
- 每条建议以 "【建议】" 开头
- 每条建议 2-3 句话，具体而不空洞
- 结合当前章节给出可落地的写作方向
- 如果还没有内容，给出如何开始写作的实用技巧

请严格按以下格式输出，每条一行：

【建议】<建议内容>
【建议】<建议内容>
..."#,
            context_data
        ),
        _ => "请提供一些创作灵感建议，涵盖角色、世界观、情节等方面。".to_string(),
    }
}

/// 解析 LLM 返回的灵感建议
fn parse_llm_response(text: &str) -> Vec<InspirationItem> {
    let mut items = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(content) = line.strip_prefix("【建议】") {
            let content = content.trim().to_string();
            if !content.is_empty() {
                // 取前 20 个字作为标题
                let title: String = content.chars().take(24).collect();
                let title = if title.len() < content.len() {
                    format!("{}…", title)
                } else {
                    title
                };
                items.push(InspirationItem { title, content });
            }
        }
    }
    items
}

/// 精选灵感卡片（LLM 不可用时的回退）
fn curated_inspiration(context_type: &str) -> Vec<InspirationItem> {
    match context_type {
        "character" => vec![
            InspirationItem {
                title: "性格反差制造张力…".to_string(),
                content: "试着给主要角色加一个与其核心性格矛盾的次要特质。例如一个勇敢无畏的战士却害怕小动物，或者一个冷血的反派却默默资助孤儿院。这种反差会让角色更立体。".to_string(),
            },
            InspirationItem {
                title: "隐藏秘密驱动叙事…".to_string(),
                content: "每个主要角色都应该有一个不愿示人的秘密。这个秘密不一定要惊天动地，但一旦被揭开，会改变读者对这个角色所有行为的理解。秘密是驱动角色行为最深层的燃料。".to_string(),
            },
            InspirationItem {
                title: "镜像角色照见主…".to_string(),
                content: "设计一个和主角形成镜像对照的角色：相似的出身但做出了不同的选择，或者拥有相同目标但手段截然相反。通过对比，主角的独特性格会自然凸显。".to_string(),
            },
            InspirationItem {
                title: "背景故事创造情感…".to_string(),
                content: "不要直接告诉读者角色的过去，而是通过细节暗示：一个伤疤、一句脱口而出的方言、一种反常的习惯。让读者在故事推进中拼凑出角色的完整背景。".to_string(),
            },
            InspirationItem {
                title: "关系网络编织戏剧…".to_string(),
                content: "在角色之间建立多层次的复杂关系，而非简单的好友或敌人。例如：A 欠 B 一条命，B 深爱 C，C 却是 A 失散多年的亲人。关系越纠缠，戏剧张力越强。".to_string(),
            },
        ],
        "world" => vec![
            InspirationItem {
                title: "法则必有代价…".to_string(),
                content: "如果你的世界有超自然规则或特殊科技，确保它们有明确的代价和限制。没有代价的力量会让故事失去张力。例如：魔法消耗寿命、高科技依赖于稀有资源、天赋需要牺牲其他能力。".to_string(),
            },
            InspirationItem {
                title: "微观细节胜过宏观…".to_string(),
                content: "与其大段描述世界的历史地理，不如通过几个日常细节让读者感受世界的质感：人们早餐吃什么、街头的告示栏贴着什么、下层阶级用什么货币。一个具体的细节比十句概括更有说服力。".to_string(),
            },
            InspirationItem {
                title: "文化与信仰塑造冲…".to_string(),
                content: "不同地区应该有不同甚至冲突的文化习俗和价值观。当来自不同地域的角色相遇时，文化差异本身就是绝佳的冲突来源，也能让世界显得真实。".to_string(),
            },
            InspirationItem {
                title: "时间线让历史活过…".to_string(),
                content: "为世界构建一个简要的时间线，标注几个关键历史事件。这些事件不必全部出现在故事中，但它们的存在会让世界有厚重的历史感。角色可以偶然提及或受到这些历史事件的影响。".to_string(),
            },
            InspirationItem {
                title: "日常感让幻想真实…".to_string(),
                content: "即使是奇幻世界，也需要有日常生活的细节：人们如何谋生、孩子怎么上学、节日怎么庆祝、生病了怎么办。这些日常感能让最离奇的设定也变得可信。".to_string(),
            },
        ],
        "outline" => vec![
            InspirationItem {
                title: "三幕结构的变奏…".to_string(),
                content: "不必死板遵循三幕结构。试试在第二幕中段加入一个「假胜利」，让主角以为目标达成，然后迅速摧毁这个假象。这种节奏变化会让读者在舒适区被打破时获得更强烈的阅读体验。".to_string(),
            },
            InspirationItem {
                title: "伏笔的「枪与餐桌…".to_string(),
                content: "契诃夫的名言适用于长篇创作：如果你在第一幕提到一把枪，它必须在第三幕之前发射。每条伏笔都需要对应回收，但回收方式可以出乎意料。做好伏笔登记表，避免遗漏。".to_string(),
            },
            InspirationItem {
                title: "悬念层级金字塔…".to_string(),
                content: "在同一时间维持多个层级的悬念：整本书的核心谜团（大悬念）、每卷要解决的中等冲突（中悬念）、每章结尾的钩子（小悬念）。让读者在解开一个小悬念的同时，被更大的悬念吸引着继续读下去。".to_string(),
            },
            InspirationItem {
                title: "人物弧光决定故事…".to_string(),
                content: "故事的核心不是情节本身，而是人物在情节中的变化。为每个主要角色规划好弧光：起点是什么状态、经历了什么、终点变成什么样。情节只是触发人物变化的催化剂。".to_string(),
            },
            InspirationItem {
                title: "场景的功能性设计…".to_string(),
                content: "每个场景至少承担两个功能中的一个：推进情节或深化人物。如果一个场景两者都不做，考虑删除或重构它。优秀的场景往往两者兼备——在推进情节的同时展现人物变化。".to_string(),
            },
        ],
        "writing" => vec![
            InspirationItem {
                title: "开场第一句定基调…".to_string(),
                content: "花最多时间打磨每章的第一句话。好的开场句应该做到三件事：建立氛围、暗示冲突、勾起好奇心。避免以天气描写或角色醒来开头，除非这些元素直接服务于叙事目的。".to_string(),
            },
            InspirationItem {
                title: "展示而非告知…".to_string(),
                content: "这是写作的第一黄金法则。不要说「他很生气」，而是写「他握紧了拳头，指节泛白」。让读者通过角色的行动、对话和生理反应来推断情绪状态，而不是直接被告知。".to_string(),
            },
            InspirationItem {
                title: "对话潜文本的力量…".to_string(),
                content: "最好的对话不是角色直抒胸臆，而是他们口是心非。角色说出来的话和他们真正想表达的意思之间的落差，就是潜文本的空间。利用对话中的停顿、回避和转移话题来制造张力。".to_string(),
            },
            InspirationItem {
                title: "节奏张弛有度…".to_string(),
                content: "长篇创作中节奏至关重要。动作场面之后需要舒缓的过渡段落让读者喘息，平静描写之后需要冲突来重新抓住注意力。可以通过句子长度来调节节奏：短句加快节奏，长句放缓节奏。".to_string(),
            },
            InspirationItem {
                title: "感官描写让世界立…".to_string(),
                content: "不要只依赖视觉描写。气味、声音、温度、质感——调动所有感官。最能让读者身临其境的，往往不是主角看到了什么，而是他闻到的气味、听到的背景音、脚底传来的触感。".to_string(),
            },
        ],
        _ => vec![
            InspirationItem {
                title: "从熟悉处寻找灵感…".to_string(),
                content: "当灵感枯竭时，回到你最喜欢的小说、电影或游戏，分析它们的结构是如何运作的。不是抄袭，而是理解「为什么这个情节奏效」，然后将同样的原理应用到自己的创作中。".to_string(),
            },
            InspirationItem {
                title: "反向思考打破惯性…".to_string(),
                content: "如果你觉得故事走向太 predictable，试试「最不可能发生的事情是什么？」把那个选项写出来，然后思考如何让它合理发生。反直觉的选择往往催生最精彩的情节。".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_response() {
        let text = "【建议】给主角加一个隐藏的秘密身份
【建议】让反派有一个令人同情的动机
无关文本
【建议】第三幕加入反转结局";

        let items = parse_llm_response(text);
        assert_eq!(items.len(), 3);
        assert!(items[0].content.contains("秘密身份"));
        assert!(items[1].content.contains("令人同情的动机"));
        assert!(items[2].content.contains("反转结局"));
    }

    #[test]
    fn test_curated_inspiration_character() {
        let items = curated_inspiration("character");
        assert!(!items.is_empty());
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_curated_inspiration_world() {
        let items = curated_inspiration("world");
        assert!(!items.is_empty());
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_curated_inspiration_outline() {
        let items = curated_inspiration("outline");
        assert!(!items.is_empty());
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_curated_inspiration_writing() {
        let items = curated_inspiration("writing");
        assert!(!items.is_empty());
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_curated_inspiration_unknown() {
        let items = curated_inspiration("unknown");
        assert!(!items.is_empty());
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_build_prompt() {
        let prompt = build_prompt("character", "{\"name\": \"Alice\"}");
        assert!(prompt.contains("角色创作"));
        assert!(prompt.contains("Alice"));
        assert!(prompt.contains("【建议】"));
    }

    #[test]
    fn test_generate_inspiration_no_provider() {
        let items = generate_inspiration(None, None, "character", "{}");
        assert!(!items.is_empty());
        // 没有 provider 时应该回退到精选内容
        assert_eq!(items.len(), 5);
    }
}
