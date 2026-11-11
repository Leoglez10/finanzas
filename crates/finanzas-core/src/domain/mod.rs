//! Pure business rules: no I/O, no clock. Callers pass `today` explicitly.

pub mod envelope;
pub mod money;
pub mod month;
pub mod recurrence;
pub mod reminders;
pub mod schedule;
pub mod summary;
