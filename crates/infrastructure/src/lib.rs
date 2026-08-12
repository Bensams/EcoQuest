//! EcoQuest infrastructure adapters (PostgreSQL today, more later).

pub mod achievement_store;
pub mod auth_store;
pub mod certificate_store;
pub mod event_store;
pub mod impact_store;
pub mod postgres;
pub mod seed;

pub use achievement_store::PgAchievementStore;
pub use auth_store::PgAuthStore;
pub use certificate_store::PgCertificateStore;
pub use event_store::PgEventStore;
pub use impact_store::PgImpactStore;
pub use postgres::PgStore;
