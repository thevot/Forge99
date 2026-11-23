use super::*;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

type AutoSaveWake = Arc<dyn Fn() + Send + Sync>;

struct AutoSaveJob {
    map_instance_id: u64,
    doc_rev: u64,
    seq: u64,
    disk_path: PathBuf,
    snapshot: crate::compiler::T3dExportSnapshot,
    cancel: Arc<AtomicBool>,
}

struct AutoSaveOutcome {
    map_instance_id: u64,
    #[allow(dead_code)]
    doc_rev: u64,
    seq: u64,
    disk_path: PathBuf,
    timestamp: String,
    t3d_str: Option<String>,
    format_error: Option<String>,
    disk_error: Option<String>,
    serialize_ms: f64,
    io_ms: f64,
}

struct AutoSaveShared {
    active: Option<(u64, u64, Arc<AtomicBool>)>,
    pending: Option<AutoSaveJob>,
    completed: Option<AutoSaveOutcome>,
    wake: Option<AutoSaveWake>,
    next_seq: u64,
    shutdown: bool,
}

pub(super) struct AsyncAutoSaveCoordinator {
    shared: Arc<(Mutex<AutoSaveShared>, Condvar)>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AsyncAutoSaveTick {
    pub dispatched: bool,
    pub completed: bool,
    pub snapshot_ms: f64,
    pub bg_serialize_ms: f64,
    pub bg_io_ms: f64,
}

#[inline]
fn utc_hh_mm_ss() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02} UTC", hours, mins, s)
}

impl AsyncAutoSaveCoordinator {
    fn new() -> Self {
        let shared = Arc::new((
            Mutex::new(AutoSaveShared {
                active: None,
                pending: None,
                completed: None,
                wake: None,
                next_seq: 1,
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let worker_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("forge99-autosave".to_string())
            .spawn(move || Self::worker_loop(worker_shared))
            .ok();
        Self { shared, handle }
    }

    fn worker_loop(shared: Arc<(Mutex<AutoSaveShared>, Condvar)>) {
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
                guard.active = Some((job.map_instance_id, job.seq, Arc::clone(&job.cancel)));
                job
            };

            if job.cancel.load(Ordering::Relaxed) {
                let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                if guard.active.as_ref().is_some_and(|a| a.1 == job.seq) {
                    guard.active = None;
                }
                continue;
            }

            let ser_t0 = Instant::now();
            let t3d_map = crate::compiler::CompilerBridge::export_t3d_map_from_snapshot(&job.snapshot);
            let mut buf = Vec::new();
            let fmt_res = ut99_format::write_t3d(&mut buf, &t3d_map);
            let serialize_ms = ser_t0.elapsed().as_secs_f64() * 1000.0;

            if job.cancel.load(Ordering::Relaxed) {
                let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                if guard.active.as_ref().is_some_and(|a| a.1 == job.seq) {
                    guard.active = None;
                }
                continue;
            }

            let timestamp = utc_hh_mm_ss();
            let (t3d_str, format_error, disk_error, io_ms) = match fmt_res {
                Err(e) => (None, Some(e.to_string()), None, 0.0),
                Ok(()) => {
                    let s = String::from_utf8_lossy(&buf).into_owned();
                    let io_t0 = Instant::now();
                    let disk_res = stage_snapshot(&job.disk_path, |_, mut file| {
                        file.write_all(s.as_bytes())?;
                        file.sync_all()?;
                        Ok(())
                    });
                    let io_ms = io_t0.elapsed().as_secs_f64() * 1000.0;
                    (Some(s), None, disk_res.err().map(|e| e.to_string()), io_ms)
                }
            };

            let wake_cb = {
                let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                if guard.active.as_ref().is_some_and(|a| a.1 == job.seq) {
                    guard.active = None;
                }
                if !guard.shutdown && !job.cancel.load(Ordering::Relaxed) && guard.pending.is_none() {
                    guard.completed = Some(AutoSaveOutcome {
                        map_instance_id: job.map_instance_id,
                        doc_rev: job.doc_rev,
                        seq: job.seq,
                        disk_path: job.disk_path,
                        timestamp,
                        t3d_str,
                        format_error,
                        disk_error,
                        serialize_ms,
                        io_ms,
                    });
                    guard.wake.clone()
                } else {
                    None
                }
            };

            if let Some(wake) = wake_cb {
                wake();
            }
        }
    }

    fn submit(
        &self,
        map_instance_id: u64,
        doc_rev: u64,
        disk_path: PathBuf,
        snapshot: crate::compiler::T3dExportSnapshot,
        wake: Option<AutoSaveWake>,
    ) -> u64 {
        let (lock, cvar) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        let seq = guard.next_seq;
        guard.next_seq = guard.next_seq.wrapping_add(1).max(1);
        if wake.is_some() {
            guard.wake = wake;
        }
        if let Some((_, _, ref cancel)) = guard.active {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(prev_pending) = guard.pending.take() {
            prev_pending.cancel.store(true, Ordering::Relaxed);
        }
        guard.completed = None;
        guard.pending = Some(AutoSaveJob {
            map_instance_id,
            doc_rev,
            seq,
            disk_path,
            snapshot,
            cancel: Arc::new(AtomicBool::new(false)),
        });
        cvar.notify_one();
        seq
    }

    fn cancel_for_other_map(&self, current_map_id: u64) {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((active_map, _, ref cancel)) = guard.active {
            if active_map != current_map_id {
                cancel.store(true, Ordering::Relaxed);
            }
        }
        if guard
            .pending
            .as_ref()
            .is_some_and(|p| p.map_instance_id != current_map_id)
        {
            if let Some(p) = guard.pending.take() {
                p.cancel.store(true, Ordering::Relaxed);
            }
        }
        if guard
            .completed
            .as_ref()
            .is_some_and(|c| c.map_instance_id != current_map_id)
        {
            guard.completed = None;
        }
    }

    fn take_completed(&self) -> Option<AutoSaveOutcome> {
        let (lock, _) = &*self.shared;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        guard.completed.take()
    }

    fn is_in_flight(&self) -> bool {
        let (lock, _) = &*self.shared;
        let guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        guard.active.is_some() || guard.pending.is_some() || guard.completed.is_some()
    }
}

impl Drop for AsyncAutoSaveCoordinator {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
            guard.shutdown = true;
            if let Some((_, _, ref cancel)) = guard.active {
                cancel.store(true, Ordering::Relaxed);
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

pub(super) fn new_session_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    std::env::temp_dir().join("forge99_autosave").join(format!(
        "session-{}-{nanos}-{}", std::process::id(), NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

struct StagedSnapshot(PathBuf);
impl Drop for StagedSnapshot {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

fn stage_snapshot(target: &Path, write: impl FnOnce(&Path, std::fs::File) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let parent = target.parent().ok_or_else(|| anyhow::anyhow!("Missing backup directory"))?;
    std::fs::create_dir_all(parent)?;
    let staged_path = parent.join(format!(".forge99-{}-{}.tmp", std::process::id(), NEXT_ID.fetch_add(1, Ordering::Relaxed)));
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged_path)?;
    let staged = StagedSnapshot(staged_path);
    write(&staged.0, file)?;
    // Same-directory rename replaces existing files on Windows and Unix.
    // Never remove the destination first: a failed commit must preserve it.
    std::fs::rename(&staged.0, target)?;
    Ok(())
}

impl EditorState {
    /// Opens a potentially corrupted `.unr` map in Safe Recovery Mode (`REQ-J06`),
    /// discarding any broken compiled `Model_World` BSP or stale `ReachSpecs` and recovering source CSG brushes & actors.
    pub fn load_unr_safe_recovery(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let recovered = ut99_format::UnrLevel::open_safe_recovery(path)?;
        self.load_unr_level(recovered, path)?;
        // Recovery discards compiled navigation, including links from the previous map.
        self.clear_ai_path_links_without_history();
        self.map_is_dirty = true;
        self.log(format!(
            "Safe Recovery Mode: Recovered {} brush(es) and {} actor(s) from '{}'.",
            self.world_brushes.len(),
            self.loaded_unr.as_ref().map_or(0, |u| u.actors.len()),
            path.display()
        ));
        Ok(())
    }


    /// Saves a rotating UnrealEd 2.0 backup file (`Auto0.unr` .. `Auto9.unr`) in `dir` (`REQ-J06`).
    pub fn save_rotating_autosave_unr(&mut self, dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let slot = self.auto_save_unr_slot;
        let file_name = format!("Auto{}.unr", slot);
        let target_path = dir.as_ref().join(&file_name);
        let level = crate::compiler::CompilerBridge::build_unr_level_from_state(self, &target_path)?;
        let bytes = level.build_package()?.to_bytes()?;
        stage_snapshot(&target_path, |path, mut file| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            let package = ut99_format::Package::open(path)?;
            let validation = ut99_format::UnrLevel::validate_package(&package);
            anyhow::ensure!(validation.is_valid, "Invalid backup: {}", validation.errors.join("; "));
            drop(package);
            Ok(())
        })?;
        self.auto_save_unr_slot = (slot + 1) % 10;
        let saved = target_path;
        let _ = self.trigger_auto_save();
        self.log(format!(
            "Rotating Auto-Save: Wrote backup slot '{}' to '{}'.",
            file_name,
            saved.display()
        ));
        Ok(saved)
    }


    /// Monotonic hash of document state (brushes, movers, static meshes, actors, level properties).
    pub fn document_revision(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        let mix_u64 = |h: &mut u64, v: u64| {
            *h ^= v;
            *h = h.wrapping_mul(0x100000001b3);
        };
        let mix_bytes = |h: &mut u64, bytes: &[u8]| {
            mix_u64(h, bytes.len() as u64);
            for &b in bytes {
                *h ^= b as u64;
                *h = h.wrapping_mul(0x100000001b3);
            }
        };

        mix_u64(&mut h, self.map_instance_id);
        mix_u64(&mut h, self.world_brushes.rev());
        mix_u64(&mut h, self.geometry_revision);
        mix_u64(&mut h, self.movers.rev());
        mix_u64(&mut h, self.static_meshes.rev());
        mix_u64(
            &mut h,
            self.loaded_unr.as_ref().map_or(0, |u| u.actors.rev()),
        );
        mix_u64(&mut h, self.map.actors.rev());
        mix_u64(&mut h, self.target_engine as u64);
        mix_bytes(&mut h, self.level_title.as_bytes());
        mix_bytes(&mut h, self.level_author.as_bytes());
        mix_bytes(&mut h, self.level_ideal_player_count.as_bytes());
        mix_bytes(&mut h, self.level_song.as_bytes());
        mix_bytes(&mut h, self.level_game_type.as_bytes());
        mix_bytes(&mut h, self.level_enter_text.as_bytes());
        mix_u64(&mut h, self.level_ambient_brightness as u64);
        mix_u64(&mut h, self.level_ambient_hue as u64);
        mix_u64(&mut h, self.level_ambient_saturation as u64);

        // Order-independent XOR mix over brush_groups so HashMap ordering does not affect hash.
        mix_u64(&mut h, self.brush_groups.len() as u64);
        let mut grp_acc = 0u64;
        for (&idx, grp) in &self.brush_groups {
            let mut gh = 0xcbf29ce484222325u64;
            mix_u64(&mut gh, idx as u64);
            mix_bytes(&mut gh, grp.as_bytes());
            grp_acc ^= gh;
        }
        mix_u64(&mut h, grp_acc);
        h
    }

    pub fn check_auto_save(&mut self, dt_secs: f32) -> bool {
        if !self.auto_save_enabled {
            return false;
        }
        self.auto_save_timer += dt_secs;
        if self.auto_save_timer >= self.auto_save_interval_secs {
            self.trigger_auto_save();
            return true;
        }
        false
    }


    pub fn auto_save_directory(&self) -> &Path { &self.auto_save_session_dir }

    pub fn trigger_auto_save(&mut self) -> String {
        // Failed attempts also wait for the normal interval before retrying.
        self.auto_save_timer = 0.0;
        let snapshot = crate::compiler::T3dExportSnapshot::capture(self);
        let t3d_map = crate::compiler::CompilerBridge::export_t3d_map_from_snapshot(&snapshot);
        let mut buf = Vec::new();
        if let Err(e) = ut99_format::write_t3d(&mut buf, &t3d_map) {
            let err_msg = format!("Auto-Save failed to format T3D: {e}");
            self.log(&err_msg);
            self.toast_error(&err_msg);
            return err_msg;
        }
        let t3d_str = String::from_utf8_lossy(&buf).to_string();
        let timestamp = utc_hh_mm_ss();

        let slot = self.auto_save_t3d_slot + 1;
        let disk_path = self.auto_save_session_dir.join(format!("Auto{slot}.t3d"));
        let disk_result = stage_snapshot(&disk_path, |_, mut file| {
            file.write_all(t3d_str.as_bytes())?;
            file.sync_all()?;
            Ok(())
        });

        self.auto_save_snapshots.push((timestamp.clone(), t3d_str));
        if self.auto_save_snapshots.len() > 5 {
            self.auto_save_snapshots.remove(0);
        }

        self.auto_save_timer = 0.0;
        // NOTE: Auto-Save is a background recovery snapshot and MUST NOT clear `self.map_is_dirty`.
        let msg = match disk_result {
            Ok(()) => {
                self.auto_save_t3d_slot = (self.auto_save_t3d_slot + 1) % 5;
                format!("Auto-Save: Snapshot captured in memory and saved to '{}' at {}", disk_path.display(), timestamp)
            }
            Err(error) => {
                let msg = format!("Auto-Save: Snapshot available in memory only; disk backup failed at '{}': {error}", disk_path.display());
                self.toast_error(&msg);
                msg
            }
        };
        self.log(&msg);
        msg
    }

    /// Captures a lightweight CoW snapshot on the UI thread and schedules disk staging in the worker.
    /// Returns the UI-thread snapshot capture duration in milliseconds.
    pub fn trigger_async_auto_save(
        &mut self,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> f64 {
        self.auto_save_timer = 0.0;
        let t0 = Instant::now();
        let snapshot = crate::compiler::T3dExportSnapshot::capture(self);
        let map_id = self.map_instance_id;
        let doc_rev = snapshot.doc_rev;
        let slot = self.auto_save_t3d_slot + 1;
        let disk_path = self.auto_save_session_dir.join(format!("Auto{slot}.t3d"));
        let worker = self
            .auto_save_worker
            .get_or_insert_with(AsyncAutoSaveCoordinator::new);
        let seq = worker.submit(map_id, doc_rev, disk_path, snapshot, wake);
        self.auto_save_latest_seq = seq;
        t0.elapsed().as_secs_f64() * 1000.0
    }

    /// Commits a completed background auto-save if it still matches `(map_instance_id, seq)`.
    /// Returns `Some((bg_serialize_ms, bg_io_ms))` when a matching completion was applied.
    pub fn poll_async_auto_save(&mut self) -> Option<(f64, f64)> {
        let worker = self.auto_save_worker.as_ref()?;
        let outcome = worker.take_completed()?;
        if outcome.map_instance_id != self.map_instance_id
            || outcome.seq != self.auto_save_latest_seq
        {
            return None;
        }
        if let Some(err) = outcome.format_error {
            let err_msg = format!("Auto-Save failed to format T3D: {err}");
            self.log(&err_msg);
            self.toast_error(&err_msg);
            return Some((outcome.serialize_ms, outcome.io_ms));
        }
        let Some(t3d_str) = outcome.t3d_str else {
            return None;
        };

        self.auto_save_snapshots.push((outcome.timestamp.clone(), t3d_str));
        if self.auto_save_snapshots.len() > 5 {
            self.auto_save_snapshots.remove(0);
        }
        // NOTE: Auto-Save is a background recovery snapshot and MUST NOT clear `self.map_is_dirty`.
        let msg = match outcome.disk_error {
            None => {
                self.auto_save_t3d_slot = (self.auto_save_t3d_slot + 1) % 5;
                format!(
                    "Auto-Save: Snapshot captured in memory and saved to '{}' at {}",
                    outcome.disk_path.display(),
                    outcome.timestamp
                )
            }
            Some(error) => {
                let msg = format!(
                    "Auto-Save: Snapshot available in memory only; disk backup failed at '{}': {error}",
                    outcome.disk_path.display()
                );
                self.toast_error(&msg);
                msg
            }
        };
        self.log(&msg);
        Some((outcome.serialize_ms, outcome.io_ms))
    }

    /// Advances the auto-save timer and drains completed background auto-saves without stalling the UI thread.
    pub fn tick_async_auto_save(
        &mut self,
        dt_secs: f32,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> AsyncAutoSaveTick {
        let mut tick = AsyncAutoSaveTick::default();
        if let Some((ser_ms, io_ms)) = self.poll_async_auto_save() {
            tick.completed = true;
            tick.bg_serialize_ms = ser_ms;
            tick.bg_io_ms = io_ms;
        }
        if !self.auto_save_enabled {
            return tick;
        }
        self.auto_save_timer += dt_secs;
        if self.auto_save_timer >= self.auto_save_interval_secs {
            tick.snapshot_ms = self.trigger_async_auto_save(wake);
            tick.dispatched = true;
        }
        tick
    }

    pub fn is_async_auto_save_in_flight(&self) -> bool {
        self.auto_save_worker
            .as_ref()
            .is_some_and(AsyncAutoSaveCoordinator::is_in_flight)
    }

    pub(super) fn cancel_stale_async_auto_save(&mut self) {
        self.auto_save_latest_seq = self.auto_save_latest_seq.wrapping_add(1);
        if let Some(ref worker) = self.auto_save_worker {
            worker.cancel_for_other_map(self.map_instance_id);
        }
    }


    pub fn restore_auto_save_snapshot(&mut self, index: usize) -> Result<(), String> {
        if index >= self.auto_save_snapshots.len() {
            return Err(format!("Invalid snapshot index #{index}"));
        }
        let (ts, t3d_data) = self.auto_save_snapshots[index].clone();
        let cursor = std::io::Cursor::new(t3d_data.as_bytes());
        let map = ut99_format::parse_t3d(cursor).map_err(|e| format!("Failed to parse snapshot T3D: {e}"))?;

        // Snapshots have no source path: restore as a copy requiring Save As.
        // Clear old geometry so an actor-only snapshot cannot inherit its brushes.
        self.world_brushes.clear();
        self.load_t3d_from_map(map, None);
        self.map_path = None;
        self.history = History::default();
        self.deselect_all();
        self.build_ai_paths();
        self.map_is_dirty = true;
        self.log(format!("Restored Auto-Save snapshot #{} ({}) as an unsaved copy; choose a destination when saving.", index + 1, ts));
        Ok(())
    }


    pub fn get_map_budget_stats(&self) -> MapBudgetStats {
        let mut stats = MapBudgetStats::default();
        stats.total_brushes = self.world_brushes.len();

        for b in &self.world_brushes {
            match b.csg_oper {
                CsgOperation::Add => stats.additive_brushes += 1,
                CsgOperation::Subtract => stats.subtractive_brushes += 1,
                _ => {}
            }
            let is_semi = b.polys.iter().any(|p| (p.flags & ut99_bsp::PF_SEMISOLID) != 0);
            let is_non = b.polys.iter().any(|p| (p.flags & ut99_bsp::PF_NOT_SOLID) != 0);
            if is_semi {
                stats.semisolid_brushes += 1;
            }
            if is_non {
                stats.nonsolid_brushes += 1;
            }
            stats.total_polygons += b.polys.len();
        }

        // In UE1 BSP compilers, BSP node count is typically ~1.4 to 1.8x the source polygon count due to node splits
        stats.estimated_bsp_nodes = (stats.total_polygons as f32 * 1.5).round() as usize;

        let mut unique_zones = std::collections::HashSet::new();
        unique_zones.insert("Zone0".to_string()); // Default Level Zone

        if let Some(ref unr) = self.loaded_unr {
            stats.total_actors = unr.actors.len();
            for a in &unr.actors {
                let cls = a.class_name.to_lowercase();
                if cls.contains("light") {
                    stats.light_actors += 1;
                } else if cls.contains("pathnode") || cls.contains("playerstart") || cls.contains("lift") || cls.contains("teleporter") || cls.contains("inventoryspot") {
                    stats.nav_actors += 1;
                } else if cls.contains("weapon") || cls.contains("ammo") || cls.contains("health") || cls.contains("armor") || cls.contains("pickup") || cls.contains("shieldbelt") || cls.contains("udamage") || cls.contains("boots") || cls.contains("shock") || cls.contains("rifle") || cls.contains("flak") || cls.contains("enforcer") || cls.contains("redeemer") || cls.contains("gun") || cls.contains("cannon") {
                    stats.pickup_actors += 1;
                } else if cls.contains("trigger") || cls.contains("specialevent") || cls.contains("dispatcher") || cls.contains("counter") {
                    stats.trigger_actors += 1;
                } else {
                    stats.other_actors += 1;
                }

                if cls.contains("zoneinfo") {
                    let zone_tag = if !a.group.is_empty() { a.group.clone() } else { a.name.clone() };
                    unique_zones.insert(zone_tag);
                }
            }
        }

        stats.total_movers = self.movers.len();
        stats.total_static_meshes = self.static_meshes.len();
        stats.total_projectors = self.projectors.len();
        stats.total_reachspecs = self.nav_graph.reach_specs.len();
        stats.total_zones = unique_zones.len();

        // UE1 Engine Architectural Maximums
        stats.node_budget_ratio = (stats.estimated_bsp_nodes as f32 / 32767.0).clamp(0.0, 1.0);
        stats.brush_budget_ratio = (stats.total_brushes as f32 / 4096.0).clamp(0.0, 1.0);
        stats.actor_budget_ratio = (stats.total_actors as f32 / 4096.0).clamp(0.0, 1.0);
        stats.reachspec_budget_ratio = (stats.total_reachspecs as f32 / 3072.0).clamp(0.0, 1.0);
        stats.zone_budget_ratio = (stats.total_zones as f32 / 64.0).clamp(0.0, 1.0);

        stats
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let path = new_session_dir();
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn state(&self) -> EditorState {
            let mut state = EditorState::default();
            state.new_map();
            state.auto_save_session_dir = self.0.clone();
            state
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn autosave_t3d_keeps_latest_five_disk_versions_and_restores() {
        let dir = TestDir::new();
        let mut state = dir.state();
        let map_path = Some(dir.0.join("original.unr"));
        state.map_path = map_path.clone();
        let mut versions = Vec::new();
        for version in 1..=12 {
            state.level_title = format!("Version{version}");
            state.map_is_dirty = true;
            assert!(state.trigger_auto_save().contains("saved to"));
            versions.push(state.auto_save_snapshots.last().unwrap().1.clone());
            assert!(state.map_is_dirty);
            assert_eq!(state.map_path, map_path);
        }
        for version in 8..=12 {
            let path = dir.0.join(format!("Auto{}.t3d", (version - 1) % 5 + 1));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), versions[version - 1]);
        }
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 5);
        state.load_t3d(dir.0.join("Auto2.t3d")).unwrap();
        assert_eq!(state.level_title, "Version12");
    }

    #[test]
    fn autosave_unr_rotates_twice_independently_of_t3d() {
        let dir = TestDir::new();
        let mut state = dir.state();
        let mut expected = std::collections::HashMap::new();
        state.map_path = Some(dir.0.join("original.unr"));
        let original_path = state.map_path.clone();
        for version in 1..=22 {
            state.level_title = format!("Version{version}");
            state.map_is_dirty = true;
            state.trigger_auto_save();
            let path = state.save_rotating_autosave_unr(&dir.0).unwrap();
            assert_eq!(path.file_name().unwrap(), format!("Auto{}.unr", (version - 1) % 10).as_str());
            expected.insert(path.clone(), std::fs::read(&path).unwrap());
            assert!(state.map_is_dirty);
            assert_eq!(state.map_path, original_path);
        }
        assert_eq!(expected.len(), 10);
        for (path, bytes) in expected {
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            let mut restored = dir.state();
            restored.load_unr(&path).unwrap();
            let slot: usize = path.file_stem().unwrap().to_str().unwrap()[4..].parse().unwrap();
            let version = if slot < 2 { slot + 21 } else { slot + 11 };
            assert_eq!(restored.level_title, format!("Version{version}"));
        }
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 15);
    }

    #[test]
    fn autosave_failure_retains_memory_retries_same_slot_after_interval() {
        let dir = TestDir::new();
        let mut state = dir.state();
        let blocked = dir.0.join("blocked");
        std::fs::write(&blocked, b"not a directory").unwrap();
        state.auto_save_session_dir = blocked.clone();
        state.auto_save_interval_secs = 60.0;
        assert!(state.check_auto_save(60.0));
        assert_eq!(state.auto_save_snapshots.len(), 1);
        assert_eq!(state.auto_save_t3d_slot, 0);
        assert_eq!(state.auto_save_timer, 0.0);
        assert!(state.toasts.last().unwrap().message.contains("memory only"));
        let toast_count = state.toasts.len();
        for _ in 0..100 { assert!(!state.check_auto_save(0.01)); }
        assert_eq!(state.toasts.len(), toast_count);
        assert!(state.save_rotating_autosave_unr(&blocked).is_err());
        assert_eq!(state.auto_save_unr_slot, 0);
        state.auto_save_session_dir = dir.0.clone();
        assert!(state.trigger_auto_save().contains("saved to"));
        assert!(dir.0.join("Auto1.t3d").exists());
        assert!(state.save_rotating_autosave_unr(&dir.0).unwrap().ends_with("Auto0.unr"));
    }

    #[test]
    fn autosave_staging_failure_preserves_previous_file_and_cleans_temp() {
        let dir = TestDir::new();
        let target = dir.0.join("Auto1.t3d");
        std::fs::write(&target, b"previous valid snapshot").unwrap();
        let result = stage_snapshot(&target, |_, mut file| {
            file.write_all(b"partial")?;
            anyhow::bail!("injected write failure")
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"previous valid snapshot");
        let blocked = dir.0.join("Auto2.t3d");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("keep"), b"keep").unwrap();
        assert!(stage_snapshot(&blocked, |_, mut file| {
            file.write_all(b"complete")?;
            Ok(())
        }).is_err());
        assert_eq!(std::fs::read(blocked.join("keep")).unwrap(), b"keep");
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn autosave_locked_destination_preserves_backup_and_retries_slot() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = TestDir::new();
        let mut state = dir.state();
        state.level_title = "Previous".into();
        state.trigger_auto_save();
        let target = dir.0.join("Auto1.t3d");
        let previous = std::fs::read(&target).unwrap();
        for _ in 0..4 { state.trigger_auto_save(); }
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&target).unwrap();
        state.level_title = "Replacement".into();
        assert!(state.trigger_auto_save().contains("memory only"));
        assert_eq!(state.auto_save_t3d_slot, 0);
        drop(lock);
        assert_eq!(std::fs::read(&target).unwrap(), previous);
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 5);
        assert!(state.trigger_auto_save().contains("saved to"));
        assert_ne!(std::fs::read(&target).unwrap(), previous);
    }

    #[test]
    fn safe_recovery_preserves_source_identity_without_writable_backups() {
        let dir = TestDir::new();
        let source = dir.0.join("DM-RecoverySource.unr");
        let original = dir.state();
        crate::compiler::CompilerBridge::save_unr_native(&original, &source).unwrap();
        let before = std::fs::read(&source).unwrap();
        let mut recovered = dir.state();
        let blocked = dir.0.join("unwritable-backups");
        std::fs::write(&blocked, b"not a directory").unwrap();
        recovered.auto_save_session_dir = blocked;
        let a = recovered.nav_graph.add_node("OldA", Vec3::ZERO, "PathNode");
        let b = recovered.nav_graph.add_node("OldB", Vec3::X, "PathNode");
        recovered.nav_graph.add_reach_spec(a, b, 1);
        recovered.refresh_ai_path_links();
        recovered.load_unr_safe_recovery(&source).unwrap();
        assert_eq!(recovered.map_path.as_ref(), Some(&source));
        assert_eq!(recovered.recent_maps, vec![source.clone()]);
        let level = recovered.loaded_unr.as_ref().unwrap();
        assert_eq!(level.map_name, "DM-RecoverySource");
        assert!(level.world_bsp.is_none());
        assert!(level.reach_specs.is_empty());
        assert!(recovered.nav_graph.nodes.iter().all(|node| node.name != "OldA" && node.name != "OldB"));
        assert!(recovered.nav_graph.reach_specs.is_empty());
        assert!(recovered.ai_path_links.is_empty());
        assert!(recovered.map_is_dirty, "Recovered source must be saved before closing");
        assert_eq!(std::fs::read(&source).unwrap(), before);
    }

    #[test]
    fn snapshot_restore_does_not_keep_another_maps_save_destination() {
        let dir = TestDir::new();
        let mut state = dir.state();
        state.map_path = Some(dir.0.join("DM-Source.unr"));
        state.level_title = "Source snapshot".into();
        state.trigger_auto_save();
        let other = dir.0.join("DM-Other.unr");
        state.map_path = Some(other.clone());
        state.level_title = "Other map".into();
        std::fs::write(&other, b"other map must remain untouched").unwrap();
        state.restore_auto_save_snapshot(0).unwrap();
        assert!(state.map_path.is_none(), "Restore must require an explicit save destination");
        assert!(state.map_is_dirty);
        assert_eq!(state.level_title, "Source snapshot");
        assert_eq!(std::fs::read(other).unwrap(), b"other map must remain untouched");
    }

    #[test]
    fn snapshot_restore_without_brushes_does_not_keep_previous_geometry() {
        let dir = TestDir::new();
        let mut state = dir.state();
        state.world_brushes.clear();
        state.trigger_auto_save();
        state.new_map();
        assert!(!state.world_brushes.is_empty());
        state.selected_brush_index = Some(0);
        state.history.push(crate::history::Action::MoveBrush {
            brush_index: 0, old_location: Vec3::ZERO, new_location: Vec3::X,
        });
        assert!(state.history.can_undo());
        state.restore_auto_save_snapshot(0).unwrap();
        assert!(!state.history.can_undo());
        assert!(state.selected_brush_index.is_none());
        assert!(state.world_brushes.is_empty());
        assert!(state.map_is_dirty);
    }

    #[test]
    fn autosave_sessions_do_not_overwrite_each_other() {
        let mut first = EditorState::default();
        let mut second = EditorState::default();
        assert_ne!(first.auto_save_directory(), second.auto_save_directory());
        let first_dir = TestDir(first.auto_save_directory().to_path_buf());
        let second_dir = TestDir(second.auto_save_directory().to_path_buf());
        first.level_title = "First".into();
        second.level_title = "Second".into();
        first.trigger_auto_save();
        let original = std::fs::read(first_dir.0.join("Auto1.t3d")).unwrap();
        for _ in 0..12 { second.trigger_auto_save(); }
        assert_eq!(std::fs::read(first_dir.0.join("Auto1.t3d")).unwrap(), original);
        assert_ne!(std::fs::read(second_dir.0.join("Auto1.t3d")).unwrap(), original);
    }

    #[test]
    fn async_autosave_writes_atomically_wakes_and_commits_on_poll() {
        let dir = TestDir::new();
        let mut state = dir.state();
        state.level_title = "AsyncSnapshotTitle".into();
        state.map_is_dirty = true;
        state.auto_save_interval_secs = 60.0;

        let woken = Arc::new(AtomicBool::new(false));
        let woken_cb = Arc::clone(&woken);
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            woken_cb.store(true, Ordering::Release);
        });

        let tick0 = state.tick_async_auto_save(60.0, Some(wake));
        assert!(tick0.dispatched);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !woken.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(woken.load(Ordering::Acquire), "async autosave worker must invoke wake callback");

        let tick1 = state.tick_async_auto_save(0.016, None);
        assert!(tick1.completed);
        assert!(!tick1.dispatched);
        assert!(state.map_is_dirty, "async autosave must preserve map_is_dirty");
        assert_eq!(state.auto_save_snapshots.len(), 1);
        assert_eq!(state.auto_save_t3d_slot, 1);
        let written = std::fs::read_to_string(dir.0.join("Auto1.t3d")).unwrap();
        assert!(written.contains("AsyncSnapshotTitle"));
        assert_eq!(written, state.auto_save_snapshots[0].1);
    }

    #[test]
    fn async_autosave_discards_stale_result_after_new_map() {
        let dir = TestDir::new();
        let mut state = dir.state();
        for i in 0..80 {
            let mut b = build_cube(format!("OldBrush_{i}"), CsgOperation::Add, Vec3::splat(64.0));
            b.location = Vec3::new(i as f32 * 96.0, 0.0, 0.0);
            state.world_brushes.push(b);
        }
        state.level_title = "OldMapToDiscard".into();

        let _ = state.trigger_async_auto_save(None);
        assert!(state.new_map());
        assert_eq!(state.auto_save_snapshots.len(), 0);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while state.is_async_auto_save_in_flight() && std::time::Instant::now() < deadline {
            if state.poll_async_auto_save().is_some() {
                panic!("stale outcome from previous map must never commit after new_map");
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(state.poll_async_auto_save().is_none());
        assert!(
            state.auto_save_snapshots.is_empty(),
            "stale snapshot must not pollute new map"
        );
    }

    #[test]
    fn async_autosave_latest_wins_under_rapid_triggers() {
        let dir = TestDir::new();
        let mut state = dir.state();
        for i in 1..=6 {
            state.level_title = format!("RapidRev{i}");
            let mut b = build_cube(format!("RapidBrush_{i}"), CsgOperation::Add, Vec3::splat(64.0));
            b.location = Vec3::new(i as f32 * 128.0, 0.0, 0.0);
            state.world_brushes.push(b);
            let _ = state.trigger_async_auto_save(None);
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut committed = false;
        while std::time::Instant::now() < deadline {
            if state.poll_async_auto_save().is_some() {
                committed = true;
                break;
            }
            if !state.is_async_auto_save_in_flight() {
                committed = state.poll_async_auto_save().is_some();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(committed, "latest async autosave must commit");
        assert!(state.poll_async_auto_save().is_none());
        assert_eq!(state.auto_save_snapshots.len(), 1);
        assert!(state.auto_save_snapshots[0].1.contains("RapidRev6"));
        let disk_text = std::fs::read_to_string(dir.0.join("Auto1.t3d")).unwrap();
        assert!(disk_text.contains("RapidRev6"));
    }
}
