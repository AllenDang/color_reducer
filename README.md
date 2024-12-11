# Color Reducer Library

Welcome to the **Color Reducer** library! This Rust crate provides functionality to simplify images by reducing the number of colors based on a predefined palette. It is particularly useful for image processing tasks where color quantization and noise reduction are required.

## Overview

The **Color Reducer** library allows you to reduce the number of colors in an image by mapping each pixel to the closest color in a predefined palette. Additionally, it offers functionality to merge small regions (considered as noise) into larger ones based on an area threshold, enhancing image quality by removing insignificant details.

## Features

- **Color Reduction**: Simplify images by reducing colors to a predefined palette.
- **Noise Reduction**: Merge small regions based on area threshold to eliminate noise.

## Installation

Add this crate to your project:

```bash
cargo add color_reducer
```

Then, run `cargo build` to fetch the library.

## Usage

### Basic Example

Here's a simple example of how to use the **Color Reducer** library to process an image:

```rust
use image::DynamicImage;
use color_reducer::ColorReducer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load an image from a file path
    let img = image::open("input_image.png")?;

    // Define your palette of colors (as RGB arrays)
    let palette = vec![
        [255, 255, 255], // White
        [0, 0, 0],       // Black
        [255, 0, 0],     // Red
        // Add more colors as needed
    ];

    // Create a new ColorReducer instance
    let reducer = ColorReducer::new(palette);

    // Process the image
    let reduced_img = reducer.reduce(&img)?;

    // Save the processed image to a file
    reduced_img.save("output_image.png")?;

    Ok(())
}
```

## Performance

- Parallel Processing: The library uses Rust's concurrency features to process the color reduction step efficiently.
- Optimized Algorithms: Algorithms are designed to be performant while maintaining code readability and reliability.

## License

This project is licensed under the MIT License and APACHE License. See the [LICENSE](LICENSE) file for details.

---

Thank you for choosing the **Color Reducer** library! We hope it meets your image processing needs. If you encounter any issues or have any questions, please feel free to reach out via the issue tracker on GitHub.
