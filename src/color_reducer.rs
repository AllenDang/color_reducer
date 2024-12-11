use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::filter::median_filter;
use palette::{cast, color_difference::EuclideanDistance, FromColor, Lab, Srgb};
use rayon::prelude::*;

pub struct ColorReducer {
    palette: Vec<[u8; 3]>,
}

impl ColorReducer {
    pub fn new(palette: Vec<[u8; 3]>) -> Self {
        ColorReducer { palette }
    }

    /// reduce image via palette
    /// merge_area_threshold: Optional, if some, it is the area of a pixel block, and any block
    /// less then this area will be merged with surrounding color.
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
                let srgb = cast::from_array_ref::<Srgb<u8>>(&rgb);
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

        // Construct a new image
        let new_image: RgbaImage = ImageBuffer::from_fn(width, height, |x, y| {
            simplified_pixels[(y * width + x) as usize]
        });

        let denoise_img = median_filter(&new_image, 3, 3);

        Ok(DynamicImage::ImageRgba8(denoise_img))
    }
}
