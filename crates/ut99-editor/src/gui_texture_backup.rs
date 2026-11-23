//! CPU copies of live managed UI textures, used only to repopulate a new device.
//! Full color images share their Arc; freed textures are removed immediately.
use std::{collections::HashMap, sync::Arc};
use egui::{ImageData, TextureId};
use egui::epaint::ImageDelta;

#[derive(Default)]
pub(crate) struct GuiTextureBackup {
    images: HashMap<TextureId, ImageDelta>,
}
impl GuiTextureBackup {
    pub fn update(&mut self, id: TextureId, delta: &ImageDelta) {
        let Some([x, y]) = delta.pos else {
            self.images.insert(id, delta.clone());
            return;
        };
        let target = self.images.get_mut(&id).expect("partial UI texture requires an existing full image");
        let width = target.image.width();
        let patch_width = delta.image.width();
        let patch_height = delta.image.height();
        assert!(x + patch_width <= width && y + patch_height <= target.image.height());
        match (&mut target.image, &delta.image) {
            (ImageData::Color(target), ImageData::Color(patch)) => {
                let target = Arc::make_mut(target);
                for row in 0..patch_height {
                    target.pixels[(y + row) * width + x..(y + row) * width + x + patch_width]
                        .copy_from_slice(&patch.pixels[row * patch_width..(row + 1) * patch_width]);
                }
            }
            (ImageData::Font(target), ImageData::Font(patch)) => {
                for row in 0..patch_height {
                    target.pixels[(y + row) * width + x..(y + row) * width + x + patch_width]
                        .copy_from_slice(&patch.pixels[row * patch_width..(row + 1) * patch_width]);
                }
            }
            _ => panic!("UI texture update changed image type"),
        }
        target.options = delta.options;
    }
    pub fn free(&mut self, id: &TextureId) { self.images.remove(id); }
    pub fn restore(&self, renderer: &mut egui_wgpu::Renderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        for (id, delta) in &self.images { renderer.update_texture(device, queue, *id, delta); }
    }
    pub fn bytes(&self) -> usize {
        self.images.values().map(|delta| delta.image.width() * delta.image.height() * 4).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_patches_keep_unmodified_pixels_and_free_releases_backup() {
        let mut backup = GuiTextureBackup::default();
        let id = TextureId::Managed(1);
        let full = Arc::new(egui::ColorImage::new([3, 2], egui::Color32::RED));
        backup.update(id, &ImageDelta::full(full.clone(), egui::TextureOptions::LINEAR));
        backup.update(id, &ImageDelta::partial([1, 1], egui::ColorImage::new([2, 1], egui::Color32::GREEN), egui::TextureOptions::NEAREST));
        let ImageData::Color(image) = &backup.images[&id].image else { panic!() };
        assert_eq!(image.pixels, vec![egui::Color32::RED, egui::Color32::RED, egui::Color32::RED,
            egui::Color32::RED, egui::Color32::GREEN, egui::Color32::GREEN]);
        assert!(full.pixels.iter().all(|pixel| *pixel == egui::Color32::RED));
        assert_eq!(backup.images[&id].pos, None);
        assert_eq!(backup.images[&id].options, egui::TextureOptions::NEAREST);
        assert_eq!(backup.bytes(), 24);
        backup.free(&id);
        assert_eq!(backup.bytes(), 0);
    }
    #[test]
    fn font_patches_and_full_replacements_replay_as_complete_images() {
        let mut backup = GuiTextureBackup::default();
        let id = TextureId::Managed(0);
        backup.update(id, &ImageDelta::full(egui::FontImage::new([4, 4]), egui::TextureOptions::LINEAR));
        let mut patch = egui::FontImage::new([1, 2]); patch.pixels.fill(0.75);
        backup.update(id, &ImageDelta::partial([2, 1], patch, egui::TextureOptions::LINEAR));
        let ImageData::Font(image) = &backup.images[&id].image else { panic!() };
        assert_eq!(image.pixels.iter().filter(|value| **value == 0.75).count(), 2);
        assert_eq!(image.pixels[6], 0.75); assert_eq!(image.pixels[10], 0.75);
        backup.update(id, &ImageDelta::full(egui::FontImage::new([8, 8]), egui::TextureOptions::LINEAR));
        assert_eq!(backup.bytes(), 256);
    }
}
