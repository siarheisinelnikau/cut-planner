//! Original geometric mark: three fitted boards and a visible saw kerf.
pub fn rgba(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (
                (x as f32 + 0.5) / size as f32,
                (y as f32 + 0.5) / size as f32,
            );
            let inside = (0.04..0.96).contains(&u)
                && (0.04..0.96).contains(&v)
                && (u - 0.5).abs().max(0.34).hypot((v - 0.5).abs().max(0.34)) < 0.591;
            let mut color = if inside {
                [24, 75, 64, 255]
            } else {
                [0, 0, 0, 0]
            };
            if (0.19..0.81).contains(&u) && (0.19..0.81).contains(&v) {
                color = if u < 0.47 {
                    [192, 227, 210, 255]
                } else if u < 0.53 || (0.47..0.53).contains(&v) {
                    [232, 164, 87, 255]
                } else if v < 0.47 {
                    [245, 239, 213, 255]
                } else {
                    [140, 193, 180, 255]
                };
            }
            pixels.extend_from_slice(&color);
        }
    }
    pixels
}

pub fn ico() -> Vec<u8> {
    let sizes = [16_u32, 20, 24, 32, 48, 64, 128, 256];
    let images: Vec<_> = sizes
        .iter()
        .map(|size| {
            let mut image = Vec::new();
            image.extend_from_slice(&40_u32.to_le_bytes());
            image.extend_from_slice(&size.to_le_bytes());
            image.extend_from_slice(&(size * 2).to_le_bytes());
            image.extend_from_slice(&1_u16.to_le_bytes());
            image.extend_from_slice(&32_u16.to_le_bytes());
            image.extend_from_slice(&[0_u8; 24]);
            let pixels = rgba(*size);
            for y in (0..*size).rev() {
                for x in 0..*size {
                    let i = ((y * size + x) * 4) as usize;
                    image.extend_from_slice(&[
                        pixels[i + 2],
                        pixels[i + 1],
                        pixels[i],
                        pixels[i + 3],
                    ]);
                }
            }
            image.resize(image.len() + (size.div_ceil(32) * 4 * size) as usize, 0);
            image
        })
        .collect();
    let mut result = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut offset = 6 + sizes.len() * 16;
    for (size, image) in sizes.iter().zip(&images) {
        result.extend_from_slice(&[*size as u8, *size as u8, 0, 0, 1, 0, 32, 0]);
        result.extend_from_slice(&(image.len() as u32).to_le_bytes());
        result.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in images {
        result.extend(image);
    }
    result
}
