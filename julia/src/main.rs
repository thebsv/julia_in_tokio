use image::{ImageBuffer, Rgb};
use itertools::Itertools;
use num_complex::Complex;
use tokio::task::JoinHandle;
use std::env;
use std::fs::File;

async fn compute_color(z0: Complex<f64>, c: Complex<f64>, iterations: u32) -> Rgb<u8> {
    let mut z = z0;
    let mut i = 0;
    while z.norm() <= 2.0 && i < iterations {
        z = z * z + c;
        i += 1;
    }
    
    let color = match i == iterations {
        true => Rgb([0, 0, 0]),
        false => {
            let r = (i as f64 / iterations as f64).powf(0.9);
            let g = (i as f64 / iterations as f64).powf(0.2);
            let b = 1.0 - (i as f64 / iterations as f64).powf(0.4);
            Rgb([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8])
        }
    };
    
    color
}

#[tokio::main]
async fn draw_fractal(
    width: u32,
    height: u32,
    iterations: u32,
    scale: f64,
    zoom: f64,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut image_buffer = ImageBuffer::new(width, height);
    let (w, h) = (width as f64, height as f64);
    let (capture_w, capture_h) = (
        (width as f64 / zoom) as u32,
        (height as f64 / zoom) as u32,
    );


    let mut thread_matrix: Vec<Vec<Option<JoinHandle<_>>>> = Vec::new();

    for x in 0..width as usize {
        thread_matrix.push(Vec::new());
        for y in 0..height as usize {
            let cx = (x as f64 - 0.5 * capture_w as f64) * scale / w;
            let cy = (y as f64 - 0.5 * capture_h as f64) * scale / h;
            let c = Complex::new(0.353343, 0.5133225);
            let z  = Complex::new(cx, cy);
            let handle = Some( tokio::spawn(async move { compute_color(z, c,  iterations) }) );
            thread_matrix[x].push(handle);
        }
    }

    for x in 0..width as usize {
        for y in 0..height as usize {
            let color: JoinHandle<_> = thread_matrix[x][y].take().unwrap();
            let res_color: Rgb<u8> = color.await.unwrap().await;
            image_buffer.put_pixel(x as u32, y as u32, res_color);
        }
    }

    image_buffer
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        println!("Usage: {} <output_file> <width>x<height> <capture_width>x<capture_height> <max_iter> <scale>", args[0]);
        return Ok(());
    }
    let output_file = &args[1];
    let (width, height) = args[2]
        .split('x')
        .map(|x| x.parse::<u32>().unwrap())
        .collect::<Vec<_>>()
        .into_iter()
        .next_tuple()
        .unwrap();

    let (capture_width, capture_height) = args[3]
        .split('x')
        .map(|x| x.parse::<u32>().unwrap())
        .collect::<Vec<_>>()
        .into_iter()
        .next_tuple()
        .unwrap();

    let iterations = args[4].parse::<u32>().unwrap();
    let scale = args[5].parse::<f64>().unwrap();
    let image_buffer = draw_fractal(
        capture_width,
        capture_height,
        iterations,
        scale,
        1.0
    );
    let resized = image::imageops::resize(
        &image_buffer,
        width,
        height,
        image::imageops::FilterType::Lanczos3,
    );
    let _ = File::create(output_file)?;
    resized.save_with_format(output_file, image::ImageFormat::Png)?;
    Ok(())
}
