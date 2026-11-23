use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct SurfaceRecovery {
    failures: u32,
    last_error: Option<wgpu::SurfaceError>,
    retry_at: Option<Instant>,
}

pub(crate) struct Retry {
    pub at: Instant,
    pub reconfigure: bool,
    pub report: bool,
}

impl SurfaceRecovery {
    pub fn pending_until(&self, now: Instant) -> Option<Instant> {
        self.retry_at.filter(|deadline| *deadline > now)
    }
    pub fn is_recovering(&self) -> bool { self.failures > 0 }
    pub fn failed(&mut self, error: wgpu::SurfaceError, now: Instant) -> Retry {
        let report = self.last_error.as_ref() != Some(&error);
        self.failures = self.failures.saturating_add(1);
        let milliseconds = if error == wgpu::SurfaceError::OutOfMemory {
            2000
        } else {
            (100_u64 << self.failures.saturating_sub(1).min(5)).min(2000)
        };
        let at = now + Duration::from_millis(milliseconds);
        self.retry_at = Some(at);
        let reconfigure = matches!(error, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated);
        self.last_error = Some(error);
        Retry { at, reconfigure, report }
    }
    pub fn succeeded(&mut self) -> bool {
        let recovered = self.is_recovering();
        *self = Self::default();
        recovered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transient_failures_back_off_and_success_resets_recovery() {
        let now = Instant::now();
        let mut recovery = SurfaceRecovery::default();
        for milliseconds in [100, 200, 400, 800, 1600, 2000, 2000] {
            let retry = recovery.failed(wgpu::SurfaceError::Timeout, now);
            assert_eq!(retry.at - now, Duration::from_millis(milliseconds));
            assert!(!retry.reconfigure);
            assert_eq!(recovery.pending_until(now), Some(retry.at));
            assert_eq!(recovery.pending_until(retry.at), None);
        }
        assert!(recovery.succeeded());
        assert!(!recovery.succeeded());
        assert_eq!(recovery.failed(wgpu::SurfaceError::Timeout, now).at - now, Duration::from_millis(100));
    }
    #[test]
    fn only_lost_and_outdated_surfaces_are_reconfigured() {
        for error in [wgpu::SurfaceError::Lost, wgpu::SurfaceError::Outdated,
            wgpu::SurfaceError::Timeout, wgpu::SurfaceError::Other, wgpu::SurfaceError::OutOfMemory] {
            let expected = matches!(error, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated);
            let mut recovery = SurfaceRecovery::default();
            let retry = recovery.failed(error.clone(), Instant::now());
            assert_eq!(retry.reconfigure, expected);
            assert!(retry.report);
            assert!(!recovery.failed(error, Instant::now()).report);
        }
    }
    #[test]
    fn memory_pressure_uses_slow_retries_immediately() {
        let now = Instant::now();
        let mut recovery = SurfaceRecovery::default();
        assert_eq!(recovery.failed(wgpu::SurfaceError::OutOfMemory, now).at - now, Duration::from_secs(2));
    }
}
