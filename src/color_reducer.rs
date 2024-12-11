use std::collections::HashMap;

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use kd_tree::KdTree;
use rayon::prelude::*;

// Union-Find Data Structure
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        UnionFind {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            // Path compression optimization
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let x_root = self.find(x);
        let y_root = self.find(y);
        if x_root != y_root {
            // Union by root
            self.parent[y_root] = x_root;
        }
    }
}

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

        let kd_palette_lab = KdTree::build_by_ordered_float(
            self.palette
                .iter()
                .map(|&rgb| {
                    let [r, g, b] = rgb;
                    [r as f32, g as f32, b as f32]
                })
                .collect(),
        );

        let simplified_pixels: Vec<Rgba<u8>> = pixels
            .par_iter()
            .map(|pixel| {
                let rgb = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];

                match kd_palette_lab.nearest(&rgb) {
                    Some(found) => {
                        let closest_color = found.item;
                        Rgba([
                            closest_color[0] as u8,
                            closest_color[1] as u8,
                            closest_color[2] as u8,
                            pixel[3],
                        ])
                    }
                    None => *pixel,
                }
            })
            .collect();

        let mut labels = vec![0usize; (width * height) as usize];
        let mut uf = UnionFind::new((width * height) as usize);
        let mut next_label = 1usize;

        // First Pass
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let current_color = simplified_pixels[idx];

                // Neighbors (up and left)
                let mut neighbor_labels = Vec::new();

                if x > 0 {
                    let left_idx = idx - 1;
                    if simplified_pixels[left_idx] == current_color {
                        neighbor_labels.push(labels[left_idx]);
                    }
                }
                if y > 0 {
                    let up_idx = idx - width as usize;
                    if simplified_pixels[up_idx] == current_color {
                        neighbor_labels.push(labels[up_idx]);
                    }
                }

                if neighbor_labels.is_empty() {
                    // Assign new label
                    labels[idx] = next_label;
                    next_label += 1;
                } else {
                    // Assign the smallest label
                    let &min_label = neighbor_labels.iter().min().unwrap();
                    labels[idx] = min_label;
                    // Record equivalences
                    for &label in &neighbor_labels {
                        uf.union(min_label, label);
                    }
                }
            }
        }

        // Second Pass
        let mut label_sizes = HashMap::new();
        (0..labels.len()).for_each(|idx| {
            if labels[idx] != 0 {
                let root_label = uf.find(labels[idx]);
                labels[idx] = root_label;
                *label_sizes.entry(root_label).or_insert(0usize) += 1;
            }
        });

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
