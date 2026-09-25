//! 2D Graphic Primitives for Pure-Rust HUD and Skeleton Rendering
//!
//! Provides thick lines, filled circles, rounded rectangles, and alpha-compositing
//! directly onto raw pixel buffers (RGBA8888 matching Android ANativeWindow_Buffer).

/// A lightweight view over a raw raster frame buffer.
pub struct FrameBuffer<'a> {
    pub buffer: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub stride_pixels: u32,
}

impl<'a> FrameBuffer<'a> {
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32, stride_pixels: u32) -> Self {
        Self {
            buffer,
            width,
            height,
            stride_pixels,
        }
    }

    /// Computes the exact byte offset for pixel (px, py) in RGBA8888.
    #[inline(always)]
    pub fn pixel_offset(&self, px: u32, py: u32) -> usize {
        ((py * self.stride_pixels + px) * 4) as usize
    }
}

/// Blends a single RGBA pixel over an existing destination buffer pixel in place.
#[inline(always)]
pub fn blend_pixel_rgba(buffer: &mut [u8], offset: usize, color: [u8; 4]) {
    if offset + 3 >= buffer.len() {
        return;
    }

    let alpha = color[3] as u32;
    if alpha == 255 {
        buffer[offset] = color[0];
        buffer[offset + 1] = color[1];
        buffer[offset + 2] = color[2];
        buffer[offset + 3] = 255;
    } else if alpha > 0 {
        let inv_alpha = 255 - alpha;
        buffer[offset] =
            ((color[0] as u32 * alpha + buffer[offset] as u32 * inv_alpha) / 255) as u8;
        buffer[offset + 1] =
            ((color[1] as u32 * alpha + buffer[offset + 1] as u32 * inv_alpha) / 255) as u8;
        buffer[offset + 2] =
            ((color[2] as u32 * alpha + buffer[offset + 2] as u32 * inv_alpha) / 255) as u8;
        buffer[offset + 3] = 255;
    }
}

/// Draws a filled rectangle with alpha transparency onto a FrameBuffer.
pub fn draw_filled_rect_rgba(
    fb: &mut FrameBuffer,
    x: i32,
    y: i32,
    rect_w: u32,
    rect_h: u32,
    color: [u8; 4],
) {
    let x_end = (x + rect_w as i32).min(fb.width as i32);
    let y_end = (y + rect_h as i32).min(fb.height as i32);
    let x_start = x.max(0);
    let y_start = y.max(0);

    for py in y_start..y_end {
        let row_start = (py as u32 * fb.stride_pixels) as usize;
        for px in x_start..x_end {
            let offset = (row_start + px as usize) * 4;
            blend_pixel_rgba(fb.buffer, offset, color);
        }
    }
}

/// Draws an outlined rectangle onto a FrameBuffer.
pub fn draw_rect_outline_rgba(
    fb: &mut FrameBuffer,
    x: i32,
    y: i32,
    rect_w: u32,
    rect_h: u32,
    thickness: i32,
    color: [u8; 4],
) {
    // Top
    draw_filled_rect_rgba(fb, x, y, rect_w, thickness as u32, color);
    // Bottom
    draw_filled_rect_rgba(
        fb,
        x,
        y + rect_h as i32 - thickness,
        rect_w,
        thickness as u32,
        color,
    );
    // Left
    draw_filled_rect_rgba(fb, x, y, thickness as u32, rect_h, color);
    // Right
    draw_filled_rect_rgba(
        fb,
        x + rect_w as i32 - thickness,
        y,
        thickness as u32,
        rect_h,
        color,
    );
}

/// Draws a filled circle at (center_x, center_y) with radius `r`.
pub fn draw_filled_circle_rgba(
    fb: &mut FrameBuffer,
    cx: i32,
    cy: i32,
    radius: i32,
    color: [u8; 4],
) {
    let r2 = radius * radius;
    let y_start = (cy - radius).max(0);
    let y_end = (cy + radius).min(fb.height as i32 - 1);

    for py in y_start..=y_end {
        let dy = py - cy;
        let dy2 = dy * dy;
        let row_start = (py as u32 * fb.stride_pixels) as usize;

        let x_start = (cx - radius).max(0);
        let x_end = (cx + radius).min(fb.width as i32 - 1);

        for px in x_start..=x_end {
            let dx = px - cx;
            if (dx * dx) + dy2 <= r2 {
                let offset = (row_start + px as usize) * 4;
                blend_pixel_rgba(fb.buffer, offset, color);
            }
        }
    }
}

/// Draws a thick line segment from (x0, y0) to (x1, y1).
pub fn draw_line_rgba(
    fb: &mut FrameBuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    thickness: i32,
    color: [u8; 4],
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let steps = dx.max(dy).max(1);

    let x_inc = (x1 - x0) as f32 / steps as f32;
    let y_inc = (y1 - y0) as f32 / steps as f32;

    let mut cur_x = x0 as f32;
    let mut cur_y = y0 as f32;

    let half_t = (thickness / 2).max(0);

    for _ in 0..=steps {
        let px = cur_x.round() as i32;
        let py = cur_y.round() as i32;

        if half_t == 0 {
            if px >= 0 && px < fb.width as i32 && py >= 0 && py < fb.height as i32 {
                let offset = ((py as u32 * fb.stride_pixels + px as u32) * 4) as usize;
                blend_pixel_rgba(fb.buffer, offset, color);
            }
        } else {
            draw_filled_circle_rgba(fb, px, py, half_t, color);
        }

        cur_x += x_inc;
        cur_y += y_inc;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_primitives() {
        let (w, h) = (100, 100);
        let stride_pixels = w;
        let mut buffer = vec![0u8; (w * h * 4) as usize];
        let mut fb = FrameBuffer::new(&mut buffer, w, h, stride_pixels);

        draw_filled_rect_rgba(&mut fb, 10, 10, 20, 20, [255, 0, 0, 255]);
        draw_filled_circle_rgba(&mut fb, 50, 50, 10, [0, 255, 0, 255]);
        draw_line_rgba(&mut fb, 0, 0, 99, 99, 2, [0, 0, 255, 255]);
        draw_rect_outline_rgba(&mut fb, 70, 70, 20, 20, 2, [255, 255, 0, 255]);

        let non_zero_count = fb.buffer.iter().filter(|&&b| b > 0).count();
        assert!(non_zero_count > 500);
    }
}
