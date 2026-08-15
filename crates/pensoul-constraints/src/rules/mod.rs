// rules/ — 内置约束规则

pub mod character;
pub mod timeline;
pub mod setting;
pub mod foreshadow;
pub mod event;

pub use character::CharacterConsistencyRule;
pub use timeline::TimelineConsistencyRule;
pub use setting::SettingConsistencyRule;
pub use foreshadow::ForeshadowTrackingRule;
pub use event::EventContinuityRule;
