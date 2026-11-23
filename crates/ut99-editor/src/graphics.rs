//! Runtime graphics selection, limits negotiation, hardware tier budgets, and frame diagnostics.
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HardwareProfile {
    #[default]
    Auto,
    High,
    Balanced,
    Constrained,
}

impl HardwareProfile {
    pub const ALL: [Self; 4] = [Self::Auto, Self::High, Self::Balanced, Self::Constrained];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::High => "High",
            Self::Balanced => "Balanced",
            Self::Constrained => "Constrained",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolvedHardwareTier {
    High,
    #[default]
    Balanced,
    Constrained,
}

impl ResolvedHardwareTier {
    pub const fn label(self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Balanced => "Balanced",
            Self::Constrained => "Constrained",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GraphicsBackendPreference {
    #[default]
    Auto,
    Vulkan,
    Dx12,
    Gl,
    Metal,
}

impl GraphicsBackendPreference {
    pub const ALL: [Self; 5] = [Self::Auto, Self::Vulkan, Self::Dx12, Self::Gl, Self::Metal];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Vulkan => "Vulkan",
            Self::Dx12 => "DirectX 12",
            Self::Gl => "OpenGL",
            Self::Metal => "Metal",
        }
    }

    pub const fn to_wgpu_backends(self) -> Option<wgpu::Backends> {
        match self {
            Self::Auto => None,
            Self::Vulkan => Some(wgpu::Backends::VULKAN),
            Self::Dx12 => Some(wgpu::Backends::DX12),
            Self::Gl => Some(wgpu::Backends::GL),
            Self::Metal => Some(wgpu::Backends::METAL),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PresentModePreference {
    #[default]
    AutoVsync,
    Fifo,
    Mailbox,
    Immediate,
}

impl PresentModePreference {
    pub const ALL: [Self; 4] = [Self::AutoVsync, Self::Fifo, Self::Mailbox, Self::Immediate];

    pub const fn label(self) -> &'static str {
        match self {
            Self::AutoVsync => "AutoVsync",
            Self::Fifo => "Fifo (VSync)",
            Self::Mailbox => "Mailbox",
            Self::Immediate => "Immediate",
        }
    }
}

pub fn resolve_present_mode(
    pref: PresentModePreference,
    supported: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    let desired = match pref {
        PresentModePreference::AutoVsync => wgpu::PresentMode::AutoVsync,
        PresentModePreference::Fifo => wgpu::PresentMode::Fifo,
        PresentModePreference::Mailbox => wgpu::PresentMode::Mailbox,
        PresentModePreference::Immediate => wgpu::PresentMode::Immediate,
    };
    if supported.contains(&desired) {
        return desired;
    }
    if supported.contains(&wgpu::PresentMode::AutoVsync) {
        return wgpu::PresentMode::AutoVsync;
    }
    if supported.contains(&wgpu::PresentMode::Fifo) {
        return wgpu::PresentMode::Fifo;
    }
    supported.first().copied().unwrap_or(wgpu::PresentMode::Fifo)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableAdapterInfo {
    pub name: String,
    pub backend: wgpu::Backend,
    pub device_type: wgpu::DeviceType,
    pub driver: String,
    pub driver_info: String,
    pub max_texture_dimension_2d: u32,
    pub max_texture_array_layers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GraphicsSelectionOptions {
    pub preferred_backend: GraphicsBackendPreference,
    pub preferred_adapter_name: Option<String>,
    pub hardware_profile: HardwareProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareBudgets {
    pub tier: ResolvedHardwareTier,
    pub diffuse_texture_layers: u32,
    pub lightmap_atlas_dim: u32,
    pub max_retained_vertex_bytes: usize,
    pub texture_manager_budget_bytes: usize,
    pub gui_texture_backup_soft_cap_bytes: usize,
}

impl HardwareBudgets {
    pub fn for_tier_and_limits(tier: ResolvedHardwareTier, limits: &wgpu::Limits) -> Self {
        let max_layers = limits.max_texture_array_layers.max(16);
        let max_dim = limits.max_texture_dimension_2d.max(256);
        match tier {
            ResolvedHardwareTier::High => Self {
                tier,
                diffuse_texture_layers: 384.min(max_layers).max(16),
                lightmap_atlas_dim: 2048.min(max_dim).max(256),
                max_retained_vertex_bytes: 32 * 1024 * 1024,
                texture_manager_budget_bytes: 256 * 1024 * 1024,
                gui_texture_backup_soft_cap_bytes: 64 * 1024 * 1024,
            },
            ResolvedHardwareTier::Balanced => Self {
                tier,
                diffuse_texture_layers: 256.min(max_layers).max(16),
                lightmap_atlas_dim: 2048.min(max_dim).max(256),
                max_retained_vertex_bytes: 16 * 1024 * 1024,
                texture_manager_budget_bytes: 160 * 1024 * 1024,
                gui_texture_backup_soft_cap_bytes: 48 * 1024 * 1024,
            },
            ResolvedHardwareTier::Constrained => Self {
                tier,
                diffuse_texture_layers: 128.min(max_layers).max(16),
                lightmap_atlas_dim: 1024.min(max_dim).max(256),
                max_retained_vertex_bytes: 8 * 1024 * 1024,
                texture_manager_budget_bytes: 96 * 1024 * 1024,
                gui_texture_backup_soft_cap_bytes: 32 * 1024 * 1024,
            },
        }
    }

    /// Default budgets preserving the pre-existing standalone `Renderer::new` capacities
    /// while safely clamping to the active device's negotiated texture limits.
    pub fn default_for_limits(limits: &wgpu::Limits) -> Self {
        let max_layers = limits.max_texture_array_layers.max(16);
        let max_dim = limits.max_texture_dimension_2d.max(256);
        Self {
            tier: ResolvedHardwareTier::Balanced,
            diffuse_texture_layers: 256.min(max_layers).max(16),
            lightmap_atlas_dim: 2048.min(max_dim).max(256),
            max_retained_vertex_bytes: 32 * 1024 * 1024,
            texture_manager_budget_bytes: 256 * 1024 * 1024,
            gui_texture_backup_soft_cap_bytes: 64 * 1024 * 1024,
        }
    }
}

pub fn resolve_hardware_tier(
    profile: HardwareProfile,
    info: &wgpu::AdapterInfo,
    limits: &wgpu::Limits,
) -> ResolvedHardwareTier {
    match profile {
        HardwareProfile::High => ResolvedHardwareTier::High,
        HardwareProfile::Balanced => ResolvedHardwareTier::Balanced,
        HardwareProfile::Constrained => ResolvedHardwareTier::Constrained,
        HardwareProfile::Auto => {
            if info.device_type == wgpu::DeviceType::Cpu
                || limits.max_texture_dimension_2d < 4096
                || limits.max_texture_array_layers < 128
            {
                ResolvedHardwareTier::Constrained
            } else if info.device_type == wgpu::DeviceType::DiscreteGpu
                && limits.max_texture_dimension_2d >= 8192
                && limits.max_texture_array_layers >= 256
            {
                ResolvedHardwareTier::High
            } else {
                ResolvedHardwareTier::Balanced
            }
        }
    }
}

/// Computes downlevel-compatible device limits clamped to the adapter's real limits,
/// unlocking higher texture array layers and dimensions when supported.
pub fn negotiate_device_limits(adapter_limits: &wgpu::Limits) -> wgpu::Limits {
    let base = wgpu::Limits::downlevel_webgl2_defaults()
        .using_resolution(adapter_limits.clone())
        .using_alignment(adapter_limits.clone());
    wgpu::Limits {
        max_texture_dimension_1d: adapter_limits.max_texture_dimension_1d.min(16384),
        max_texture_dimension_2d: adapter_limits.max_texture_dimension_2d.min(16384),
        max_texture_dimension_3d: adapter_limits.max_texture_dimension_3d.min(2048),
        max_texture_array_layers: adapter_limits.max_texture_array_layers.min(512),
        max_bind_groups: base.max_bind_groups.min(adapter_limits.max_bind_groups),
        max_bindings_per_bind_group: base
            .max_bindings_per_bind_group
            .min(adapter_limits.max_bindings_per_bind_group)
            .min(1000),
        max_dynamic_uniform_buffers_per_pipeline_layout: base
            .max_dynamic_uniform_buffers_per_pipeline_layout
            .min(adapter_limits.max_dynamic_uniform_buffers_per_pipeline_layout),
        max_sampled_textures_per_shader_stage: base
            .max_sampled_textures_per_shader_stage
            .min(adapter_limits.max_sampled_textures_per_shader_stage),
        max_samplers_per_shader_stage: base
            .max_samplers_per_shader_stage
            .min(adapter_limits.max_samplers_per_shader_stage),
        max_uniform_buffers_per_shader_stage: base
            .max_uniform_buffers_per_shader_stage
            .min(adapter_limits.max_uniform_buffers_per_shader_stage),
        max_uniform_buffer_binding_size: adapter_limits
            .max_uniform_buffer_binding_size
            .min(64 << 10),
        max_vertex_buffers: base.max_vertex_buffers.min(adapter_limits.max_vertex_buffers),
        max_buffer_size: adapter_limits.max_buffer_size.min(256 << 20),
        max_vertex_attributes: base
            .max_vertex_attributes
            .min(adapter_limits.max_vertex_attributes),
        max_vertex_buffer_array_stride: base
            .max_vertex_buffer_array_stride
            .min(adapter_limits.max_vertex_buffer_array_stride),
        min_uniform_buffer_offset_alignment: base
            .min_uniform_buffer_offset_alignment
            .max(adapter_limits.min_uniform_buffer_offset_alignment),
        min_storage_buffer_offset_alignment: base
            .min_storage_buffer_offset_alignment
            .max(adapter_limits.min_storage_buffer_offset_alignment),
        max_inter_stage_shader_components: base
            .max_inter_stage_shader_components
            .min(adapter_limits.max_inter_stage_shader_components),
        max_color_attachments: base
            .max_color_attachments
            .min(adapter_limits.max_color_attachments),
        max_color_attachment_bytes_per_sample: base
            .max_color_attachment_bytes_per_sample
            .min(adapter_limits.max_color_attachment_bytes_per_sample),
        max_non_sampler_bindings: base
            .max_non_sampler_bindings
            .min(adapter_limits.max_non_sampler_bindings)
            .min(1_000_000),
        ..base
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderFrameStats {
    pub signature_ms: f64,
    pub csg_ms: f64,
    pub wireframe_ms: f64,
    pub solids_ms: f64,
    pub overlays_ms: f64,
    pub ovl_movers_meshes_ms: f64,
    pub ovl_actors_links_ms: f64,
    pub ovl_trajectories_ms: f64,
    pub ovl_radii_portals_sky_ms: f64,
    pub ovl_zone_seals_ms: f64,
    pub ovl_dispatchers_movtraj_ms: f64,
    pub ovl_studios_gizmos_ms: f64,
    pub upload_ms: f64,
    pub encode_submit_ms: f64,
    pub csg_rebuilt: bool,
    pub csg_in_flight: bool,
    pub csg_rebuild_reason: &'static str,
    pub uploaded_line_bytes: usize,
    pub uploaded_solid_bytes: usize,
    pub line_vertex_count: usize,
    pub solid_vertex_count: usize,
    pub draw_calls: u32,
    pub texture_layers_used: usize,
    pub texture_fallback_surfaces: usize,
    pub lightmap_layers_used: usize,
    pub lightmap_fallback_surfaces: usize,
    pub solid_chunks_total: u32,
    pub solid_chunks_drawn: u32,
    pub solid_tris_total: u32,
    pub solid_tris_drawn: u32,
    pub line_chunks_total: u32,
    pub line_chunks_drawn: u32,
    pub line_segs_total: u32,
    pub line_segs_drawn: u32,
}

#[derive(Default)]
pub struct GraphicsDiagnostics {
    pub adapter: Option<wgpu::AdapterInfo>,
    pub adapter_max_texture_array_layers: u32,
    pub adapter_max_texture_dimension_2d: u32,
    pub resolved_tier: ResolvedHardwareTier,
    pub negotiated_max_texture_dimension_2d: u32,
    pub negotiated_max_texture_array_layers: u32,
    pub diffuse_texture_layer_cap: u32,
    pub diffuse_texture_array_bytes: usize,
    pub lightmap_atlas_dim: u32,
    pub lightmap_atlas_bytes: usize,
    pub retained_vertex_budget_bytes: usize,
    pub texture_manager_used_bytes: usize,
    pub texture_manager_budget_bytes: usize,
    pub gui_texture_backup_soft_cap_bytes: usize,
    pub available_adapters: Vec<AvailableAdapterInfo>,
    pub supported_present_modes: Vec<wgpu::PresentMode>,
    pub active_present_mode: Option<wgpu::PresentMode>,
    pub active_max_frame_latency: u32,
    pub active_render_scale: f32,
    pub active_render_resolution: (u32, u32),
    pub monitor_refresh_hz: f32,
    pub gpu_timing_supported: bool,
    pub gpu_viewport_timing: Option<(usize, f64, f64)>,
    pub cpu_acquire_ms: f64,
    pub cpu_ui_ms: f64,
    pub cpu_world_ms: f64,
    pub cpu_overlay_ms: f64,
    pub cpu_present_ms: f64,
    pub cpu_autosave_ms: f64,
    pub bg_autosave_serialize_ms: f64,
    pub bg_autosave_io_ms: f64,
    pub failed_attempts: Vec<String>,
    pub cpu_vertex_storage_bytes: usize,
    pub cpu_grid_cache_bytes: usize,
    pub gpu_vertex_buffer_bytes: usize,
    pub lightmap_layers_used: usize,
    pub lightmap_fallback_surfaces: usize,
    pub csg_rebuilds_total: u64,
    pub last_csg_rebuild_reason: &'static str,
    pub last_render_stats: RenderFrameStats,
    pub device_losses: u64,
    pub device_recoveries: u64,
    pub last_device_error: Option<String>,
    pub gui_texture_backup_bytes: usize,
    pub surface_failures: u64,
    pub surface_recoveries: u64,
    pub last_surface_error: Option<String>,
    pub frames_over_50ms: u64,
    pub frames_over_100ms: u64,
    pub max_frame_ms: f64,
    frames: VecDeque<f64>,
}

impl GraphicsDiagnostics {
    pub fn record_frame(&mut self, milliseconds: f64) {
        if self.frames.len() == 120 { self.frames.pop_front(); }
        self.frames.push_back(milliseconds);
        if milliseconds > 50.0 {
            self.frames_over_50ms = self.frames_over_50ms.saturating_add(1);
        }
        if milliseconds > 100.0 {
            self.frames_over_100ms = self.frames_over_100ms.saturating_add(1);
        }
        if milliseconds.is_finite() && milliseconds > self.max_frame_ms {
            self.max_frame_ms = milliseconds;
        }
    }

    /// CPU wall time including acquire/present waits; this is not GPU execution time.
    pub fn frame_percentiles_ms(&self) -> Option<(f64, f64)> {
        if self.frames.is_empty() { return None; }
        let mut samples: Vec<_> = self.frames.iter().copied().collect();
        samples.sort_by(f64::total_cmp);
        Some((samples[(samples.len() - 1) / 2], samples[(samples.len() - 1) * 95 / 100]))
    }

    /// Returns `(p50, p95, p99, max_in_window)` across the recent 120-frame window.
    pub fn frame_summary_ms(&self) -> Option<(f64, f64, f64, f64)> {
        if self.frames.is_empty() { return None; }
        let mut samples: Vec<_> = self.frames.iter().copied().collect();
        samples.sort_by(f64::total_cmp);
        let last = samples.len() - 1;
        Some((
            samples[last / 2],
            samples[last * 95 / 100],
            samples[last * 99 / 100],
            samples[last],
        ))
    }

    pub fn format_diagnostics_report(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "=== FORGE99 GRAPHICS & FRAME DIAGNOSTICS ===");
        if let Some(adapter) = &self.adapter {
            let _ = writeln!(
                out,
                "Adapter: {} | Backend: {:?} | Type: {:?} | Driver: {} {}",
                adapter.name, adapter.backend, adapter.device_type, adapter.driver, adapter.driver_info
            );
        } else {
            let _ = writeln!(out, "Adapter: not initialized");
        }
        let _ = writeln!(
            out,
            "Hardware Tier: {} | Adapter Limits: tex2d={} array_layers={} | Negotiated: tex2d={} array_layers={}",
            self.resolved_tier.label(),
            self.adapter_max_texture_dimension_2d,
            self.adapter_max_texture_array_layers,
            self.negotiated_max_texture_dimension_2d,
            self.negotiated_max_texture_array_layers,
        );
        if self.active_present_mode.is_some()
            || self.monitor_refresh_hz > 0.0
            || self.active_render_resolution.0 > 0
        {
            let scale_pct = if self.active_render_scale > 0.0 {
                (self.active_render_scale * 100.0).round() as u32
            } else {
                100
            };
            let refresh_hz = if self.monitor_refresh_hz > 0.0 {
                self.monitor_refresh_hz
            } else {
                60.0
            };
            let _ = writeln!(
                out,
                "Display/Presentation: PresentMode={:?} max_frame_latency={} render_scale={}% render_res={}x{} monitor={:.1} Hz",
                self.active_present_mode,
                self.active_max_frame_latency,
                scale_pct,
                self.active_render_resolution.0,
                self.active_render_resolution.1,
                refresh_hz,
            );
        }
        if let Some((p50, p95, p99, max_win)) = self.frame_summary_ms() {
            let _ = writeln!(
                out,
                "Frame Wall (last 120): p50={p50:.2} ms p95={p95:.2} ms p99={p99:.2} ms win_max={max_win:.2} ms | session_max={:.2} ms | >50ms={} >100ms={}",
                self.max_frame_ms, self.frames_over_50ms, self.frames_over_100ms
            );
        }
        let r = &self.last_render_stats;
        let _ = writeln!(
            out,
            "CPU Last Frame (ms): acquire={:.2} ui={:.2} world={:.2} [sig={:.2} csg={:.2} wire={:.2} solid={:.2} ovl={:.2} up={:.2} sub={:.2}] overlay={:.2} present={:.2} autosave={:.2}",
            self.cpu_acquire_ms, self.cpu_ui_ms, self.cpu_world_ms,
            r.signature_ms, r.csg_ms, r.wireframe_ms, r.solids_ms, r.overlays_ms, r.upload_ms, r.encode_submit_ms,
            self.cpu_overlay_ms, self.cpu_present_ms, self.cpu_autosave_ms
        );
        let _ = writeln!(
            out,
            "Overlay Breakdown (ms): mov_mesh={:.2} actors_links={:.2} traj={:.2} rad_port_sky={:.2} zone_seals={:.2} disp_movt={:.2} studios_giz={:.2}",
            r.ovl_movers_meshes_ms,
            r.ovl_actors_links_ms,
            r.ovl_trajectories_ms,
            r.ovl_radii_portals_sky_ms,
            r.ovl_zone_seals_ms,
            r.ovl_dispatchers_movtraj_ms,
            r.ovl_studios_gizmos_ms,
        );
        if let Some((count, p50, p95)) = self.gpu_viewport_timing {
            let _ = writeln!(out, "GPU 3D Pass (samples={count}): p50={p50:.3} ms p95={p95:.3} ms");
        }
        if self.cpu_autosave_ms > 0.0 || self.bg_autosave_serialize_ms > 0.0 || self.bg_autosave_io_ms > 0.0 {
            let _ = writeln!(
                out,
                "Autosave (ms): snapshot={:.2} | bg_serialize={:.2} | bg_io={:.2}",
                self.cpu_autosave_ms, self.bg_autosave_serialize_ms, self.bg_autosave_io_ms
            );
        }
        let _ = writeln!(
            out,
            "Render Counters: draw_calls={} line_verts={} solid_verts={} upload_line={} B upload_solid={} B csg_rebuilds={} csg_in_flight={} last_reason={}",
            r.draw_calls, r.line_vertex_count, r.solid_vertex_count,
            r.uploaded_line_bytes, r.uploaded_solid_bytes,
            self.csg_rebuilds_total, r.csg_in_flight, self.last_csg_rebuild_reason
        );
        let _ = writeln!(
            out,
            "Textures/Lightmaps: diffuse_layers={}/{} diffuse_fallbacks={} ({:.2} MiB) | lm_tiles={} dim={} ({:.2} MiB) lm_fallbacks={}",
            r.texture_layers_used,
            self.diffuse_texture_layer_cap.saturating_sub(1),
            r.texture_fallback_surfaces,
            self.diffuse_texture_array_bytes as f64 / 1_048_576.0,
            r.lightmap_layers_used,
            self.lightmap_atlas_dim,
            self.lightmap_atlas_bytes as f64 / 1_048_576.0,
            r.lightmap_fallback_surfaces,
        );
        let _ = writeln!(
            out,
            "Memory (MiB): cpu_verts={:.2}/{:.2} cpu_grid={:.2} gpu_verts={:.2} tex_mgr={:.2}/{:.2} ui_backup={:.2}/{:.2}",
            self.cpu_vertex_storage_bytes as f64 / 1_048_576.0,
            self.retained_vertex_budget_bytes as f64 / 1_048_576.0,
            self.cpu_grid_cache_bytes as f64 / 1_048_576.0,
            self.gpu_vertex_buffer_bytes as f64 / 1_048_576.0,
            self.texture_manager_used_bytes as f64 / 1_048_576.0,
            self.texture_manager_budget_bytes as f64 / 1_048_576.0,
            self.gui_texture_backup_bytes as f64 / 1_048_576.0,
            self.gui_texture_backup_soft_cap_bytes as f64 / 1_048_576.0,
        );
        out
    }
}

fn adapter_rank(kind: wgpu::DeviceType) -> u8 {
    match kind {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    }
}

fn backend_rank(backend: wgpu::Backend) -> u8 {
    match backend {
        wgpu::Backend::Vulkan => 0,
        wgpu::Backend::Dx12 => 1,
        wgpu::Backend::Metal => 2,
        wgpu::Backend::Gl => 3,
        wgpu::Backend::BrowserWebGpu => 4,
        wgpu::Backend::Empty => 5,
    }
}

/// Returns `(backends, allow_hardware, allow_cpu)` stages in priority order:
/// optional preferred backend first, then `PRIMARY HW -> SECONDARY HW -> PRIMARY CPU -> SECONDARY CPU`.
pub fn build_fallback_stages(preferred_backend: GraphicsBackendPreference) -> Vec<(wgpu::Backends, bool, bool)> {
    let mut stages = Vec::new();
    let mut push_unique = |stage: (wgpu::Backends, bool, bool)| {
        if !stages.contains(&stage) {
            stages.push(stage);
        }
    };
    if let Some(pref) = preferred_backend.to_wgpu_backends() {
        push_unique((pref, true, false));
        push_unique((pref, false, true));
    }
    push_unique((wgpu::Backends::PRIMARY, true, false));
    push_unique((wgpu::Backends::SECONDARY, true, false));
    push_unique((wgpu::Backends::PRIMARY, false, true));
    push_unique((wgpu::Backends::SECONDARY, false, true));
    stages
}

fn adapter_matches_filter(
    info: &wgpu::AdapterInfo,
    allow_hardware: bool,
    allow_cpu: bool,
    preferred_name: Option<&str>,
) -> bool {
    let is_cpu = info.device_type == wgpu::DeviceType::Cpu;
    if is_cpu && !allow_cpu {
        return false;
    }
    if !is_cpu && !allow_hardware {
        return false;
    }
    if let Some(target) = preferred_name {
        let needle = target.trim();
        if !needle.is_empty() {
            let name_lc = info.name.to_ascii_lowercase();
            let needle_lc = needle.to_ascii_lowercase();
            if name_lc != needle_lc && !name_lc.contains(&needle_lc) {
                return false;
            }
        }
    }
    true
}

async fn try_create_device_for_adapter(
    adapter: &wgpu::Adapter,
    failed_attempts: &mut Vec<String>,
) -> Option<(wgpu::Device, wgpu::Queue, bool, wgpu::Limits)> {
    let info = adapter.get_info();
    let adapter_limits = adapter.limits();
    let negotiated = negotiate_device_limits(&adapter_limits);
    let downlevel_fallback = wgpu::Limits::downlevel_webgl2_defaults()
        .using_resolution(adapter_limits.clone())
        .using_alignment(adapter_limits.clone());
    let optional_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    let feature_attempts = if optional_features.is_empty() {
        vec![wgpu::Features::empty()]
    } else {
        vec![optional_features, wgpu::Features::empty()]
    };
    let limit_attempts = if negotiated == downlevel_fallback {
        vec![negotiated]
    } else {
        vec![negotiated, downlevel_fallback]
    };

    for limits in &limit_attempts {
        for &features in &feature_attempts {
            match adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Forge99 graphics device"),
                        required_features: features,
                        required_limits: limits.clone(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
            {
                Ok((device, queue)) => {
                    let negotiated_limits = device.limits();
                    return Some((
                        device,
                        queue,
                        features.contains(wgpu::Features::TIMESTAMP_QUERY),
                        negotiated_limits,
                    ));
                }
                Err(error) => {
                    let message = format!("{} ({:?}, {:?}): {}", info.name, info.backend, features, error);
                    tracing::warn!("Graphics adapter initialization failed: {message}");
                    failed_attempts.push(message);
                }
            }
        }
    }
    None
}

fn populate_device_diagnostics(
    diagnostics: &mut GraphicsDiagnostics,
    info: wgpu::AdapterInfo,
    adapter_limits: &wgpu::Limits,
    negotiated_limits: &wgpu::Limits,
    gpu_timing_supported: bool,
    profile: HardwareProfile,
) -> HardwareBudgets {
    let tier = resolve_hardware_tier(profile, &info, negotiated_limits);
    let budgets = HardwareBudgets::for_tier_and_limits(tier, negotiated_limits);
    diagnostics.adapter_max_texture_array_layers = adapter_limits.max_texture_array_layers;
    diagnostics.adapter_max_texture_dimension_2d = adapter_limits.max_texture_dimension_2d;
    diagnostics.resolved_tier = tier;
    diagnostics.negotiated_max_texture_dimension_2d = negotiated_limits.max_texture_dimension_2d;
    diagnostics.negotiated_max_texture_array_layers = negotiated_limits.max_texture_array_layers;
    diagnostics.diffuse_texture_layer_cap = budgets.diffuse_texture_layers;
    diagnostics.lightmap_atlas_dim = budgets.lightmap_atlas_dim;
    diagnostics.retained_vertex_budget_bytes = budgets.max_retained_vertex_bytes;
    diagnostics.texture_manager_budget_bytes = budgets.texture_manager_budget_bytes;
    diagnostics.gui_texture_backup_soft_cap_bytes = budgets.gui_texture_backup_soft_cap_bytes;
    diagnostics.gpu_timing_supported = gpu_timing_supported;
    diagnostics.adapter = Some(info);
    budgets
}

/// Enumerates compatible adapters across Primary (Vulkan/DX12/Metal) and Secondary (OpenGL)
/// backends on separate instances without attaching any window surface.
pub fn enumerate_available_adapters() -> Vec<AvailableAdapterInfo> {
    let mut found = Vec::<AvailableAdapterInfo>::new();
    for backends in [wgpu::Backends::PRIMARY, wgpu::Backends::SECONDARY] {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        for adapter in instance.enumerate_adapters(backends) {
            let info = adapter.get_info();
            let limits = adapter.limits();
            let duplicate = found.iter().any(|existing| {
                existing.name == info.name
                    && existing.backend == info.backend
                    && existing.device_type == info.device_type
            });
            if !duplicate {
                found.push(AvailableAdapterInfo {
                    name: info.name,
                    backend: info.backend,
                    device_type: info.device_type,
                    driver: info.driver,
                    driver_info: info.driver_info,
                    max_texture_dimension_2d: limits.max_texture_dimension_2d,
                    max_texture_array_layers: limits.max_texture_array_layers,
                });
            }
        }
    }
    found.sort_by(|a, b| {
        (adapter_rank(a.device_type), backend_rank(a.backend), &a.name)
            .cmp(&(adapter_rank(b.device_type), backend_rank(b.backend), &b.name))
    });
    found
}

/// Try every compatible adapter on the supplied `instance`, preferring hardware to software rendering.
/// Negotiates device limits compatible with downlevel GPUs while unlocking higher texture array caps when available.
pub async fn initialize_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue, GraphicsDiagnostics), String> {
    let mut adapters = instance.enumerate_adapters(wgpu::Backends::all());
    adapters.retain(|adapter| surface.map_or(true, |surface| adapter.is_surface_supported(surface)));
    adapters.sort_by_key(|adapter| {
        let info = adapter.get_info();
        (adapter_rank(info.device_type), backend_rank(info.backend), info.name)
    });
    let mut diagnostics = GraphicsDiagnostics::default();
    for adapter in adapters {
        let info = adapter.get_info();
        if let Some(surface) = surface {
            let capabilities = surface.get_capabilities(&adapter);
            if capabilities.formats.is_empty() || capabilities.alpha_modes.is_empty() {
                diagnostics
                    .failed_attempts
                    .push(format!("{}: no usable surface configuration", info.name));
                continue;
            }
        }
        let adapter_limits = adapter.limits();
        if let Some((device, queue, ts_supported, negotiated_limits)) =
            try_create_device_for_adapter(&adapter, &mut diagnostics.failed_attempts).await
        {
            tracing::info!(
                name = %info.name,
                backend = ?info.backend,
                device_type = ?info.device_type,
                driver = %info.driver,
                "Selected graphics adapter"
            );
            populate_device_diagnostics(
                &mut diagnostics,
                info,
                &adapter_limits,
                &negotiated_limits,
                ts_supported,
                HardwareProfile::Auto,
            );
            return Ok((adapter, device, queue, diagnostics));
        }
    }
    Err(format!(
        "No compatible graphics device could be initialized. Update your graphics driver.\n{}",
        diagnostics.failed_attempts.join("\n")
    ))
}

/// Initializes a headless graphics device using the multi-backend fallback ladder
/// (`PRIMARY HW -> SECONDARY HW -> PRIMARY CPU -> SECONDARY CPU`).
pub async fn initialize_headless_device_with_options(
    options: &GraphicsSelectionOptions,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue, GraphicsDiagnostics, HardwareBudgets), String> {
    let stages = build_fallback_stages(options.preferred_backend);
    let mut diagnostics = GraphicsDiagnostics::default();

    let name_passes: Vec<Option<&str>> = match options.preferred_adapter_name.as_deref() {
        Some(name) if !name.trim().is_empty() => vec![Some(name), None],
        _ => vec![None],
    };

    for filter_name in name_passes {
        for &(backends, allow_hardware, allow_cpu) in &stages {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends,
                ..Default::default()
            });
            let mut adapters = instance.enumerate_adapters(backends);
            adapters.retain(|adapter| {
                adapter_matches_filter(&adapter.get_info(), allow_hardware, allow_cpu, filter_name)
            });
            adapters.sort_by_key(|adapter| {
                let info = adapter.get_info();
                (adapter_rank(info.device_type), backend_rank(info.backend), info.name)
            });
            for adapter in adapters {
                let info = adapter.get_info();
                let adapter_limits = adapter.limits();
                if let Some((device, queue, ts_supported, negotiated_limits)) =
                    try_create_device_for_adapter(&adapter, &mut diagnostics.failed_attempts).await
                {
                    let budgets = populate_device_diagnostics(
                        &mut diagnostics,
                        info,
                        &adapter_limits,
                        &negotiated_limits,
                        ts_supported,
                        options.hardware_profile,
                    );
                    return Ok((adapter, device, queue, diagnostics, budgets));
                }
            }
        }
    }

    Err(format!(
        "No compatible headless graphics device could be initialized.\n{}",
        diagnostics.failed_attempts.join("\n")
    ))
}

pub struct InitializedWindowGraphics {
    pub surface: wgpu::Surface<'static>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub diagnostics: GraphicsDiagnostics,
    pub budgets: HardwareBudgets,
}

/// Initializes window surface and GPU device with multi-backend fallback.
/// Each backend stage uses a fresh `wgpu::Instance` and drops failed surfaces cleanly before retrying.
pub async fn initialize_window_graphics(
    window: Arc<winit::window::Window>,
    options: &GraphicsSelectionOptions,
) -> Result<InitializedWindowGraphics, String> {
    let stages = build_fallback_stages(options.preferred_backend);
    let mut diagnostics = GraphicsDiagnostics::default();

    let name_passes: Vec<Option<&str>> = match options.preferred_adapter_name.as_deref() {
        Some(name) if !name.trim().is_empty() => vec![Some(name), None],
        _ => vec![None],
    };

    for filter_name in name_passes {
        for &(backends, allow_hardware, allow_cpu) in &stages {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends,
                ..Default::default()
            });
            let surface = match instance.create_surface(window.clone()) {
                Ok(surface) => surface,
                Err(error) => {
                    diagnostics
                        .failed_attempts
                        .push(format!("{backends:?} surface creation failed: {error}"));
                    continue;
                }
            };
            let mut adapters = instance.enumerate_adapters(backends);
            adapters.retain(|adapter| {
                adapter.is_surface_supported(&surface)
                    && adapter_matches_filter(
                        &adapter.get_info(),
                        allow_hardware,
                        allow_cpu,
                        filter_name,
                    )
            });
            adapters.sort_by_key(|adapter| {
                let info = adapter.get_info();
                (adapter_rank(info.device_type), backend_rank(info.backend), info.name)
            });
            for adapter in adapters {
                let info = adapter.get_info();
                let capabilities = surface.get_capabilities(&adapter);
                if capabilities.formats.is_empty() || capabilities.alpha_modes.is_empty() {
                    diagnostics
                        .failed_attempts
                        .push(format!("{}: no usable surface configuration", info.name));
                    continue;
                }
                let adapter_limits = adapter.limits();
                if let Some((device, queue, ts_supported, negotiated_limits)) =
                    try_create_device_for_adapter(&adapter, &mut diagnostics.failed_attempts).await
                {
                    tracing::info!(
                        name = %info.name,
                        backend = ?info.backend,
                        device_type = ?info.device_type,
                        driver = %info.driver,
                        "Selected graphics adapter"
                    );
                    let budgets = populate_device_diagnostics(
                        &mut diagnostics,
                        info,
                        &adapter_limits,
                        &negotiated_limits,
                        ts_supported,
                        options.hardware_profile,
                    );
                    return Ok(InitializedWindowGraphics {
                        surface,
                        adapter,
                        device,
                        queue,
                        diagnostics,
                        budgets,
                    });
                }
            }
            drop(surface);
        }
    }

    Err(format!(
        "No compatible graphics device could be initialized. Update your graphics driver.\n{}",
        diagnostics.failed_attempts.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_adapter_info(device_type: wgpu::DeviceType) -> wgpu::AdapterInfo {
        wgpu::AdapterInfo {
            name: "Synthetic Test Adapter".to_string(),
            vendor: 0,
            device: 0,
            device_type,
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
        }
    }

    #[test]
    #[ignore = "requires an available graphics adapter"]
    fn initializes_real_device_with_ranked_selection() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let (_, device, queue, diagnostics) = pollster::block_on(initialize_device(&instance, None))
            .expect("graphics initialization");
        let adapter = diagnostics.adapter.expect("selected adapter diagnostics");
        eprintln!("Selected: {adapter:?}");
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        queue.submit([encoder.finish()]);
        device.poll(wgpu::Maintain::Wait);
    }

    #[test]
    fn hardware_precedes_software() {
        use wgpu::DeviceType::*;
        assert!(adapter_rank(DiscreteGpu) < adapter_rank(IntegratedGpu));
        for hardware in [DiscreteGpu, IntegratedGpu, VirtualGpu, Other] {
            assert!(adapter_rank(hardware) < adapter_rank(Cpu));
        }
    }

    #[test]
    fn timings_are_bounded_and_report_recent_frames() {
        let mut diagnostics = GraphicsDiagnostics::default();
        assert_eq!(diagnostics.frame_percentiles_ms(), None);
        for _ in 0..120 { diagnostics.record_frame(1000.0); }
        for _ in 0..120 { diagnostics.record_frame(2.0); }
        assert_eq!(diagnostics.frames.len(), 120);
        assert_eq!(diagnostics.frame_percentiles_ms(), Some((2.0, 2.0)));
    }

    #[test]
    fn summary_tracks_p99_spikes_and_thresholds() {
        let mut diagnostics = GraphicsDiagnostics::default();
        assert_eq!(diagnostics.frame_summary_ms(), None);
        for i in 1..=100 {
            diagnostics.record_frame(i as f64);
        }
        diagnostics.record_frame(125.0);
        let (p50, p95, p99, win_max) = diagnostics.frame_summary_ms().unwrap();
        assert!(p50 <= p95 && p95 <= p99 && p99 <= win_max);
        assert_eq!(win_max, 125.0);
        assert_eq!(diagnostics.max_frame_ms, 125.0);
        assert_eq!(diagnostics.frames_over_50ms, 51); // 51..=100 (50) + 125.0 (1)
        assert_eq!(diagnostics.frames_over_100ms, 1);
        let report = diagnostics.format_diagnostics_report();
        assert!(report.contains("FORGE99 GRAPHICS & FRAME DIAGNOSTICS"));
        assert!(report.contains(">50ms=51 >100ms=1"));
    }

    #[test]
    fn negotiate_device_limits_never_exceeds_downlevel_adapter() {
        let downlevel = wgpu::Limits::downlevel_webgl2_defaults();
        let negotiated = negotiate_device_limits(&downlevel);
        assert!(
            negotiated.check_limits(&downlevel),
            "negotiated limits must stay strictly within WebGL2 downlevel adapter limits"
        );
        assert_eq!(negotiated.max_texture_dimension_2d, 2048);
        assert_eq!(negotiated.max_texture_array_layers, 256);

        let tiny_adapter = wgpu::Limits {
            max_texture_dimension_2d: 2048,
            max_texture_array_layers: 128,
            max_uniform_buffer_binding_size: 16 << 10,
            max_buffer_size: 128 << 20,
            min_uniform_buffer_offset_alignment: 512,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        };
        let negotiated_tiny = negotiate_device_limits(&tiny_adapter);
        assert!(
            negotiated_tiny.check_limits(&tiny_adapter),
            "negotiated limits must never exceed constrained adapter limits"
        );
        assert_eq!(negotiated_tiny.max_texture_array_layers, 128);
        assert_eq!(negotiated_tiny.min_uniform_buffer_offset_alignment, 512);

        let workstation = wgpu::Limits {
            max_texture_dimension_2d: 32768,
            max_texture_array_layers: 2048,
            ..wgpu::Limits::default()
        };
        let negotiated_ws = negotiate_device_limits(&workstation);
        assert!(negotiated_ws.check_limits(&workstation));
        assert_eq!(negotiated_ws.max_texture_dimension_2d, 16384);
        assert_eq!(negotiated_ws.max_texture_array_layers, 512);
    }

    #[test]
    fn resolve_hardware_tier_and_budgets_match_all_tiers_and_clamp_to_limits() {
        let rich = wgpu::Limits {
            max_texture_dimension_2d: 16384,
            max_texture_array_layers: 512,
            ..wgpu::Limits::default()
        };
        assert_eq!(
            resolve_hardware_tier(HardwareProfile::Auto, &synthetic_adapter_info(wgpu::DeviceType::DiscreteGpu), &rich),
            ResolvedHardwareTier::High
        );
        assert_eq!(
            resolve_hardware_tier(HardwareProfile::Auto, &synthetic_adapter_info(wgpu::DeviceType::IntegratedGpu), &rich),
            ResolvedHardwareTier::Balanced
        );
        assert_eq!(
            resolve_hardware_tier(HardwareProfile::Auto, &synthetic_adapter_info(wgpu::DeviceType::Cpu), &rich),
            ResolvedHardwareTier::Constrained
        );

        // Downlevel limits force Constrained even on non-CPU device when arrays/textures are small.
        let tiny = wgpu::Limits {
            max_texture_dimension_2d: 2048,
            max_texture_array_layers: 64,
            ..wgpu::Limits::downlevel_webgl2_defaults()
        };
        assert_eq!(
            resolve_hardware_tier(HardwareProfile::Auto, &synthetic_adapter_info(wgpu::DeviceType::DiscreteGpu), &tiny),
            ResolvedHardwareTier::Constrained
        );
        // Explicit override takes precedence.
        assert_eq!(
            resolve_hardware_tier(HardwareProfile::High, &synthetic_adapter_info(wgpu::DeviceType::Cpu), &rich),
            ResolvedHardwareTier::High
        );

        let b_high = HardwareBudgets::for_tier_and_limits(ResolvedHardwareTier::High, &rich);
        assert_eq!(b_high.diffuse_texture_layers, 384);
        assert_eq!(b_high.lightmap_atlas_dim, 2048);
        assert_eq!(b_high.max_retained_vertex_bytes, 32 * 1024 * 1024);
        assert_eq!(b_high.texture_manager_budget_bytes, 256 * 1024 * 1024);

        let b_bal = HardwareBudgets::for_tier_and_limits(ResolvedHardwareTier::Balanced, &rich);
        assert_eq!(b_bal.diffuse_texture_layers, 256);
        assert_eq!(b_bal.lightmap_atlas_dim, 2048);
        assert_eq!(b_bal.max_retained_vertex_bytes, 16 * 1024 * 1024);
        assert_eq!(b_bal.texture_manager_budget_bytes, 160 * 1024 * 1024);

        let b_con = HardwareBudgets::for_tier_and_limits(ResolvedHardwareTier::Constrained, &rich);
        assert_eq!(b_con.diffuse_texture_layers, 128);
        assert_eq!(b_con.lightmap_atlas_dim, 1024);
        assert_eq!(b_con.max_retained_vertex_bytes, 8 * 1024 * 1024);
        assert_eq!(b_con.texture_manager_budget_bytes, 96 * 1024 * 1024);

        // Clamp check under tiny adapter limits.
        let b_clamped = HardwareBudgets::for_tier_and_limits(ResolvedHardwareTier::High, &tiny);
        assert_eq!(b_clamped.diffuse_texture_layers, 64);
        assert_eq!(b_clamped.lightmap_atlas_dim, 2048);
    }

    #[test]
    fn fallback_stage_plan_orders_primary_hw_then_secondary_hw_then_cpu() {
        assert_eq!(
            build_fallback_stages(GraphicsBackendPreference::Auto),
            vec![
                (wgpu::Backends::PRIMARY, true, false),
                (wgpu::Backends::SECONDARY, true, false),
                (wgpu::Backends::PRIMARY, false, true),
                (wgpu::Backends::SECONDARY, false, true),
            ]
        );
        let gl_stages = build_fallback_stages(GraphicsBackendPreference::Gl);
        assert_eq!(gl_stages[0], (wgpu::Backends::GL, true, false));
        assert_eq!(gl_stages[1], (wgpu::Backends::GL, false, true));
        assert!(gl_stages.contains(&(wgpu::Backends::PRIMARY, true, false)));
        assert!(gl_stages.contains(&(wgpu::Backends::PRIMARY, false, true)));
    }

    #[test]
    fn present_mode_resolution_falls_back_safely() {
        assert_eq!(
            resolve_present_mode(
                PresentModePreference::Mailbox,
                &[wgpu::PresentMode::Fifo, wgpu::PresentMode::AutoVsync]
            ),
            wgpu::PresentMode::AutoVsync
        );
        assert_eq!(
            resolve_present_mode(
                PresentModePreference::Immediate,
                &[wgpu::PresentMode::Fifo]
            ),
            wgpu::PresentMode::Fifo
        );
        assert_eq!(
            resolve_present_mode(
                PresentModePreference::Mailbox,
                &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo]
            ),
            wgpu::PresentMode::Mailbox
        );
    }
}
