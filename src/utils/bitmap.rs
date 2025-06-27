use base64::{engine::general_purpose, Engine as _};
use std::io::Cursor;

// A simple bitmap representation
pub struct Bitmap {
    // Width and height of the bitmap
    width: usize,
    height: usize,
    // Image data in RGBA format (4 bytes per pixel)
    data: Vec<u8>,
    // Scaling factor to map machine coordinates to pixels
    scale: f32,
    // Origin point in the bitmap (center by default)
    origin_x: usize,
    origin_y: usize,
}

impl Bitmap {
    pub fn new(width: usize, height: usize, scale: f32) -> Self {
        // Initialize with transparent white background
        let mut data = vec![255, 255, 255, 0]; // RGBA: transparent white
        data.resize(width * height * 4, 0);

        Bitmap {
            width,
            height,
            data,
            scale,
            origin_x: width / 2,
            origin_y: height / 2,
        }
    }

    pub fn update_pos(&mut self, x: f32, y: f32) {
        // Update the origin point based on the new position
        self.origin_x = (self.width as f32 / 2.0 + x * self.scale) as usize;
        self.origin_y = (self.height as f32 / 2.0 - y * self.scale) as usize;
    }

    // Set a pixel at machine coordinates (will be translated to bitmap coordinates)
    pub fn set_pixel(&mut self, x: f32, y: f32, z: f32) {
        // Convert machine coordinates to bitmap pixel coordinates
        let px = (self.origin_x as f32 + x * self.scale) as usize;
        let py = (self.origin_y as f32 + y * self.scale) as usize;

        // Check bounds
        if px >= self.width || py >= self.height {
            println!("Pixel out of bounds: ({}, {})", px, py);
            return;
        }

        // Determine color based on z position using HSL color wheel approach
        let (r, g, b, a) = {
            // Map z from range -3.0 to 5.0 to hue angle 0° to 360°
            // TODO: Adjust the range based on z values
            let normalized_z = (z + 3.0) / 8.0; // Now 0.0 to 1.0
            let hue = (normalized_z * 360.0) % 360.0;

            // Convert HSL to RGB (using fixed saturation and lightness for clarity)
            let saturation = 0.8; // High saturation for vivid colors
            let lightness = 0.5f32; // Medium lightness for good visibility

            // Simplified HSL to RGB conversion
            let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
            let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
            let m = lightness - c / 2.0;

            // Calculate RGB based on hue segment
            let (r, g, b) = if hue < 60.0 {
                (c, x, 0.0)
            } else if hue < 120.0 {
                (x, c, 0.0)
            } else if hue < 180.0 {
                (0.0, c, x)
            } else if hue < 240.0 {
                (0.0, x, c)
            } else if hue < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };

            // Convert to 0-255 range with full opacity
            (
                ((r + m) * 255.0) as u8,
                ((g + m) * 255.0) as u8,
                ((b + m) * 255.0) as u8,
                255,
            )
        };

        // Calculate pixel index in the data array
        let idx = (py * self.width + px) * 4;

        // Set the color
        if idx + 3 < self.data.len() {
            self.data[idx] = r;
            self.data[idx + 1] = g;
            self.data[idx + 2] = b;
            self.data[idx + 3] = a;
        }
    }

    pub fn to_data_url(&self) -> String {
        // Create a new PNG encoder
        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(
                Cursor::new(&mut png_data),
                self.width as u32,
                self.height as u32,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&self.data).unwrap();
        }

        // Convert PNG to base64
        let base64_data = general_purpose::STANDARD.encode(&png_data);

        // Return as data URL
        format!("data:image/png;base64,{}", base64_data)
    }

    // Clear the bitmap (set all pixels to transparent)
    pub fn clear(&mut self) {
        for i in 0..self.data.len() / 4 {
            let idx = i * 4;
            self.data[idx] = 255;
            self.data[idx + 1] = 255;
            self.data[idx + 2] = 255;
            self.data[idx + 3] = 0;
        }
    }
}
