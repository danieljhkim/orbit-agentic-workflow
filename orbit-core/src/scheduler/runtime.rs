use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::{OrbitError, OrbitRuntime};

#[derive(Debug, Clone, Copy)]
pub struct SchedulerRuntimeConfig {
    pub idle_sleep: Duration,
    pub max_sleep: Duration,
}

impl Default for SchedulerRuntimeConfig {
    fn default() -> Self {
        Self {
            idle_sleep: Duration::from_secs(30),
            max_sleep: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerTickResult {
    pub ran: usize,
    pub next_wake_at: Option<DateTime<Utc>>,
}

pub trait ShutdownSignal {
    fn should_stop(&self) -> bool;
}

pub struct SchedulerRuntime<'a> {
    runtime: &'a OrbitRuntime,
    config: SchedulerRuntimeConfig,
}

impl<'a> SchedulerRuntime<'a> {
    pub fn new(runtime: &'a OrbitRuntime, config: SchedulerRuntimeConfig) -> Self {
        Self { runtime, config }
    }

    pub fn tick_once(&self, now: DateTime<Utc>) -> Result<SchedulerTickResult, OrbitError> {
        let ran = self.runtime.run_due_schedulers(now)?;
        let next_wake_at = self
            .runtime
            .context
            .scheduler_store
            .next_due_scheduler_time()?;
        Ok(SchedulerTickResult { ran, next_wake_at })
    }

    pub fn run_forever(&self, shutdown: &dyn ShutdownSignal) -> Result<(), OrbitError> {
        loop {
            if shutdown.should_stop() {
                return Ok(());
            }

            let now = Utc::now();
            let tick = self.tick_once(now)?;

            if shutdown.should_stop() {
                return Ok(());
            }

            thread::sleep(self.sleep_duration_after_tick(now, tick.next_wake_at));
        }
    }

    fn sleep_duration_after_tick(
        &self,
        now: DateTime<Utc>,
        next_wake_at: Option<DateTime<Utc>>,
    ) -> Duration {
        match next_wake_at {
            Some(next_wake_at) if next_wake_at > now => {
                let sleep_for = (next_wake_at - now)
                    .to_std()
                    .unwrap_or(self.config.idle_sleep);
                sleep_for.min(self.config.max_sleep)
            }
            Some(_) | None => self.config.idle_sleep,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdue_scheduler_uses_idle_sleep_to_avoid_busy_spin() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let scheduler_runtime = SchedulerRuntime::new(
            &runtime,
            SchedulerRuntimeConfig {
                idle_sleep: Duration::from_secs(7),
                max_sleep: Duration::from_secs(60),
            },
        );
        let now = Utc::now();

        let sleep_for = scheduler_runtime.sleep_duration_after_tick(now, Some(now));

        assert_eq!(sleep_for, Duration::from_secs(7));
    }

    #[test]
    fn future_scheduler_wake_is_capped_by_max_sleep() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let scheduler_runtime = SchedulerRuntime::new(
            &runtime,
            SchedulerRuntimeConfig {
                idle_sleep: Duration::from_secs(5),
                max_sleep: Duration::from_secs(30),
            },
        );
        let now = Utc::now();

        let sleep_for = scheduler_runtime
            .sleep_duration_after_tick(now, Some(now + chrono::Duration::hours(1)));

        assert_eq!(sleep_for, Duration::from_secs(30));
    }
}
