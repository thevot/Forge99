use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

pub(crate) const ASYNC_CSG_BRUSH_THRESHOLD: usize = 64;
pub(crate) const ASYNC_CSG_POLY_THRESHOLD: usize = 512;

#[inline]
pub(crate) fn should_run_async_csg(brushes: &[ut99_bsp::Brush]) -> bool {
    if brushes.len() >= ASYNC_CSG_BRUSH_THRESHOLD {
        return true;
    }
    let mut poly_sum = 0usize;
    for b in brushes {
        poly_sum += b.polys.len();
        if poly_sum >= ASYNC_CSG_POLY_THRESHOLD {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CsgJobKey {
    pub map_instance_id: u64,
    pub csg_signature: u64,
}

pub(crate) struct CsgJobResult {
    pub key: CsgJobKey,
    pub carved_polys: Vec<(usize, ut99_bsp::Polygon)>,
    pub compute_ms: f64,
}

type AsyncCsgWake = Arc<dyn Fn() + Send + Sync>;

struct CsgJob {
    key: CsgJobKey,
    brushes: Arc<Vec<ut99_bsp::Brush>>,
    cancel: Arc<AtomicBool>,
}

struct AsyncCsgShared {
    active: Option<(CsgJobKey, Arc<AtomicBool>)>,
    pending: Option<CsgJob>,
    completed: Option<CsgJobResult>,
    wake: Option<AsyncCsgWake>,
    cancelled_total: u64,
    shutdown: bool,
}

/// Persistent single-slot latest-wins CSG worker with cooperative `AtomicBool` cancellation.
pub(crate) struct AsyncCsgCoordinator {
    shared: Arc<(Mutex<AsyncCsgShared>, Condvar)>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AsyncCsgCoordinator {
    pub(crate) fn new(wake: Option<AsyncCsgWake>) -> Self {
        let shared = Arc::new((
            Mutex::new(AsyncCsgShared {
                active: None,
                pending: None,
                completed: None,
                wake,
                cancelled_total: 0,
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let worker_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("forge99-csg-coord".to_string())
            .spawn(move || Self::worker_loop(worker_shared))
            .ok();
        Self { shared, handle }
    }

    fn worker_loop(shared: Arc<(Mutex<AsyncCsgShared>, Condvar)>) {
        let (lock, cvar) = &*shared;
        loop {
            let job = {
                let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                while !guard.shutdown && guard.pending.is_none() {
                    guard = cvar.wait(guard).unwrap_or_else(|p| p.into_inner());
                }
                if guard.shutdown {
                    return;
                }
                let Some(job) = guard.pending.take() else {
                    continue;
                };
                guard.active = Some((job.key, Arc::clone(&job.cancel)));
                job
            };

            let t0 = Instant::now();
            let result = ut99_bsp::evaluate_csg_brushes_indexed_cancellable(&job.brushes, &job.cancel);
            let compute_ms = t0.elapsed().as_secs_f64() * 1000.0;

            let wake_cb = {
                let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                if guard.active.as_ref().is_some_and(|a| a.0 == job.key) {
                    guard.active = None;
                }
                match result {
                    Some(carved_polys)
                        if !guard.shutdown
                            && !job.cancel.load(Ordering::Relaxed)
                            && guard.pending.is_none() =>
                    {
                        guard.completed = Some(CsgJobResult {
                            key: job.key,
                            carved_polys,
                            compute_ms,
                        });
                        guard.wake.clone()
                    }
                    _ => None,
                }
            };

            if let Some(wake) = wake_cb {
                wake();
            }
        }
    }

    pub(crate) fn set_wake(&self, wake: Option<AsyncCsgWake>) {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        guard.wake = wake;
    }

    pub(crate) fn request(&self, key: CsgJobKey, brushes: Arc<Vec<ut99_bsp::Brush>>) {
        let (lock, cvar) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        if guard.completed.as_ref().is_some_and(|c| c.key == key) {
            return;
        }
        guard.completed = None;

        if guard.pending.as_ref().is_some_and(|p| p.key == key) {
            return;
        }
        if guard
            .active
            .as_ref()
            .is_some_and(|(ak, ac)| *ak == key && !ac.load(Ordering::Relaxed))
        {
            if let Some(stale) = guard.pending.take() {
                if !stale.cancel.swap(true, Ordering::Relaxed) {
                    guard.cancelled_total += 1;
                }
            }
            return;
        }

        if let Some((_, ref active_cancel)) = guard.active {
            if !active_cancel.swap(true, Ordering::Relaxed) {
                guard.cancelled_total += 1;
            }
        }
        if let Some(old_pending) = guard.pending.take() {
            if !old_pending.cancel.swap(true, Ordering::Relaxed) {
                guard.cancelled_total += 1;
            }
        }
        guard.pending = Some(CsgJob {
            key,
            brushes,
            cancel: Arc::new(AtomicBool::new(false)),
        });
        cvar.notify_one();
    }

    pub(crate) fn take_matching_result(&self, expected: CsgJobKey) -> Option<CsgJobResult> {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        let done = guard.completed.take()?;
        if done.key == expected {
            Some(done)
        } else {
            None
        }
    }

    pub(crate) fn cancel_for_other_map(&self, current_map_id: u64) {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((active_key, ref active_cancel)) = guard.active {
            if active_key.map_instance_id != current_map_id
                && !active_cancel.swap(true, Ordering::Relaxed)
            {
                guard.cancelled_total += 1;
            }
        }
        if guard
            .pending
            .as_ref()
            .is_some_and(|p| p.key.map_instance_id != current_map_id)
        {
            if let Some(stale) = guard.pending.take() {
                if !stale.cancel.swap(true, Ordering::Relaxed) {
                    guard.cancelled_total += 1;
                }
            }
        }
        if guard
            .completed
            .as_ref()
            .is_some_and(|c| c.key.map_instance_id != current_map_id)
        {
            guard.completed = None;
        }
    }

    pub(crate) fn cancel_if_busy(&self) {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((_, ref active_cancel)) = guard.active {
            if !active_cancel.swap(true, Ordering::Relaxed) {
                guard.cancelled_total += 1;
            }
        }
        if let Some(stale) = guard.pending.take() {
            if !stale.cancel.swap(true, Ordering::Relaxed) {
                guard.cancelled_total += 1;
            }
        }
        guard.completed = None;
    }

    pub(crate) fn cancelled_total(&self) -> u64 {
        let (lock, _) = &*self.shared;
        lock.lock().unwrap_or_else(|p| p.into_inner()).cancelled_total
    }
}

impl Drop for AsyncCsgCoordinator {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
            guard.shutdown = true;
            if let Some((_, ref ac)) = guard.active {
                ac.store(true, Ordering::Relaxed);
            }
            if let Some(p) = guard.pending.take() {
                p.cancel.store(true, Ordering::Relaxed);
            }
            guard.completed = None;
            cvar.notify_all();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use ut99_bsp::{build_cube, CsgOperation};

    fn sample_large_scene(offset: f32) -> Vec<ut99_bsp::Brush> {
        let mut brushes = Vec::with_capacity(80);
        let mut room = build_cube("Room", CsgOperation::Subtract, Vec3::new(4096.0, 4096.0, 512.0));
        room.location = Vec3::ZERO;
        brushes.push(room);
        for i in 0..79 {
            let mut b = build_cube(format!("B_{i}"), CsgOperation::Add, Vec3::new(128.0, 128.0, 256.0));
            b.location = Vec3::new(
                ((i % 9) as f32 - 4.0) * 192.0 + offset,
                ((i / 9) as f32 - 4.0) * 192.0,
                0.0,
            );
            brushes.push(b);
        }
        brushes
    }

    #[test]
    fn threshold_separates_small_from_complex_scenes() {
        let small = vec![build_cube("One", CsgOperation::Subtract, Vec3::splat(256.0))];
        assert!(!should_run_async_csg(&small));
        let large = sample_large_scene(0.0);
        assert!(should_run_async_csg(&large));
    }

    #[test]
    fn async_csg_completes_wakes_and_matches_sync_output() {
        let woken = Arc::new(AtomicBool::new(false));
        let woken_cb = Arc::clone(&woken);
        let coord = AsyncCsgCoordinator::new(Some(Arc::new(move || {
            woken_cb.store(true, Ordering::Release);
        })));
        let brushes = Arc::new(sample_large_scene(0.0));
        let key = CsgJobKey {
            map_instance_id: 1,
            csg_signature: 42,
        };

        coord.request(key, Arc::clone(&brushes));
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        while !woken.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(woken.load(Ordering::Acquire), "async CSG must trigger wake callback");
        let done = coord
            .take_matching_result(key)
            .expect("completed job must match requested key");
        let expected = ut99_bsp::evaluate_csg_brushes_indexed(&brushes);
        assert_eq!(done.carved_polys, expected);
    }

    #[test]
    fn rapid_requests_and_map_switch_cancel_stale_jobs() {
        let coord = AsyncCsgCoordinator::new(None);
        for i in 1..=6u64 {
            let brushes = Arc::new(sample_large_scene(i as f32 * 32.0));
            coord.request(
                CsgJobKey {
                    map_instance_id: 10,
                    csg_signature: 100 + i,
                },
                brushes,
            );
        }
        assert!(coord.cancelled_total() > 0, "rapid edits must cancel prior active/pending jobs");

        let final_key = CsgJobKey {
            map_instance_id: 10,
            csg_signature: 106,
        };
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        let done = loop {
            if let Some(res) = coord.take_matching_result(final_key) {
                break res;
            }
            assert!(Instant::now() < deadline, "timed out waiting for latest key");
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        assert_eq!(done.key, final_key);

        // Switching to map 11 must reject stale results from map 10.
        coord.request(
            CsgJobKey {
                map_instance_id: 10,
                csg_signature: 999,
            },
            Arc::new(sample_large_scene(5.0)),
        );
        coord.cancel_for_other_map(11);
        assert!(
            coord
                .take_matching_result(CsgJobKey {
                    map_instance_id: 10,
                    csg_signature: 999,
                })
                .is_none()
        );
    }
}
