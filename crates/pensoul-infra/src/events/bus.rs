// bus.rs — 事件发布/订阅总线

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 事件回调类型
type EventHandler = dyn Fn(&serde_json::Value) + Send + Sync;
/// 单个事件的回调列表
type HandlerList = Vec<Arc<Box<EventHandler>>>;
/// 订阅表：事件名 → 回调列表（Arc 便于无锁克隆后回调）
type HandlerTable = HashMap<String, HandlerList>;

/// 事件总线
pub struct EventBus {
    handlers: Arc<Mutex<HandlerTable>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 订阅事件
    pub fn on<F>(&self, event_name: &str, handler: F)
    where
        F: Fn(&serde_json::Value) + Send + Sync + 'static,
    {
        // 锁中毒（panic 后）继续复用数据：事件订阅本身不维护需要一致性的内部不变量
        let mut handlers = self.handlers.lock().unwrap_or_else(|p| p.into_inner());
        handlers
            .entry(event_name.to_string())
            .or_default()
            .push(Arc::new(Box::new(handler)));
    }

    /// 发布事件
    pub fn emit(&self, event_name: &str, data: serde_json::Value) {
        // 先浅拷贝 handler 列表并释放锁，再逐个回调：
        // 避免回调内再次 emit/on 时对本锁加锁造成死锁
        let handlers: HandlerList = {
            let map = self.handlers.lock().unwrap_or_else(|p| p.into_inner());
            map.get(event_name).cloned().unwrap_or_default()
        };
        for handler in handlers {
            handler(&data);
        }
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
        }
    }
}
