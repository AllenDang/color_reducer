use std::collections::{HashMap, VecDeque};

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use palette::{cast, color_difference::EuclideanDistance, FromColor, Lab, Srgb};
use rayon::prelude::*;

pub struct ColorReducer {
    palette: Vec<[u8; 3]>,
    area_threshold: usize,
}

impl ColorReducer {
    pub fn new(palette: Vec<[u8; 3]>, area_threshold: usize) -> Self {
        ColorReducer {
            palette,
            area_threshold,
        }
    }

    pub fn reduce(&self, img: &DynamicImage) -> Result<DynamicImage, Box<dyn std::error::Error>> {
        if self.palette.is_empty() {
            return Err("Palette is empty".into());
        }

        let rgba_image = img.to_rgba8();
        let (width, height) = rgba_image.dimensions();

        // Convert the pixel data of the image into a parallel iterator
        let pixels: Vec<Rgba<u8>> = rgba_image.pixels().copied().collect();

        // Process the pixel data, replacing it with colors from the palette
        let palette_lab: Vec<Lab> = self
            .palette
            .iter()
            .map(|&rgb| {
                let srgb = cast::from_array_ref::<Srgb<u8>>(&rgb);
                Lab::from_color(srgb.into_linear())
            })
            .collect();

        let simplified_pixels: Vec<Rgba<u8>> = pixels
            .par_iter()
            .map(|pixel| {
                let rgb = [pixel[0], pixel[1], pixel[2]];
                let srgb = Srgb::new(
                    rgb[0] as f32 / 255.0,
                    rgb[1] as f32 / 255.0,
                    rgb[2] as f32 / 255.0,
                );
                let lab = Lab::from_color(srgb.into_linear());

                // Find the closest palette color
                let closest_option = palette_lab
                    .iter()
                    .zip(self.palette.iter())
                    .map(|(palette_lab, &palette_rgb)| {
                        let distance = lab.distance(*palette_lab);
                        (palette_rgb, distance)
                    })
                    .min_by(|(_, dist1), (_, dist2)| dist1.total_cmp(dist2));

                match closest_option {
                    Some((closest_color, _)) => Rgba([
                        closest_color[0],
                        closest_color[1],
                        closest_color[2],
                        pixel[3],
                    ]),
                    None => {
                        // Handle cases where the iterator is empty, for example returning the original pixel or a default color
                        // Here we return the original pixel
                        *pixel
                    }
                }
            })
            .collect();

        // Step 1: Connected component labeling
        let mut labels = vec![0u32; (width * height) as usize]; // Store the label for each pixel
        let mut label = 1u32; // Labels start from 1, 0 indicates not visited
        let mut label_sizes = HashMap::new(); // Store the area size corresponding to each label

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                if labels[idx] == 0 {
                    // Not yet labeled, start a new region
                    let current_color = simplified_pixels[idx];
                    let mut queue = VecDeque::new();
                    queue.push_back((x, y));
                    labels[idx] = label;
                    let mut size = 1usize;

                    while let Some((cx, cy)) = queue.pop_front() {
                        // Check the adjacent pixels in four directions
                        let neighbors = [
                            (cx.wrapping_sub(1), cy), // left
                            (cx + 1, cy),             // right
                            (cx, cy.wrapping_sub(1)), // up
                            (cx, cy + 1),             // down
                        ];

                        for &(nx, ny) in &neighbors {
                            if nx < width && ny < height {
                                let n_idx = (ny * width + nx) as usize;
                                if labels[n_idx] == 0 && simplified_pixels[n_idx] == current_color {
                                    labels[n_idx] = label;
                                    queue.push_back((nx, ny));
                                    size += 1;
                                }
                            }
                        }
                    }

                    // Record the area size
                    label_sizes.insert(label, size);
                    label += 1;
                }
            }
        }

        // Step 2: Region merging
        // Create a new pixel buffer to store the final pixel values
        let mut final_pixels = simplified_pixels.clone();

        // Traverse all the labels and process regions with an area smaller than the threshold
        for (&current_label, &size) in &label_sizes {
            if size < self.area_threshold {
                // Small regions that need to be merged
                // Find all the pixel indices of that region
                let mut region_indices = Vec::new();
                for (idx, &label_value) in labels.iter().enumerate() {
                    if label_value == current_label {
                        region_indices.push(idx);
                    }
                }

                // Find all the boundary pixels of that region
                let mut neighbor_colors = HashMap::new();
                for &idx in &region_indices {
                    let x = (idx as u32) % width;
                    let y = (idx as u32) / width;

                    let neighbors = [
                        (x.wrapping_sub(1), y), // left
                        (x + 1, y),             // right
                        (x, y.wrapping_sub(1)), // up
                        (x, y + 1),             // down
                    ];

                    for &(nx, ny) in &neighbors {
                        if nx < width && ny < height {
                            let n_idx = (ny * width + nx) as usize;
                            let neighbor_label = labels[n_idx];
                            if neighbor_label != current_label {
                                let neighbor_color = simplified_pixels[n_idx];
                                *neighbor_colors.entry(neighbor_color).or_insert(0) += 1;
                            }
                        }
                    }
                }

                // Find the most frequently occurring neighbor color
                if let Some((&most_common_color, _)) =
                    neighbor_colors.iter().max_by_key(|&(_, &count)| count)
                {
                    // Replace the color of all pixels in the area with the neighbor color
                    for &idx in &region_indices {
                        final_pixels[idx] = most_common_color;
                    }
                }
            }
        }

        // Construct a new image
        let new_image: RgbaImage =
            ImageBuffer::from_fn(width, height, |x, y| final_pixels[(y * width + x) as usize]);

        Ok(DynamicImage::ImageRgba8(new_image))
    }
}
