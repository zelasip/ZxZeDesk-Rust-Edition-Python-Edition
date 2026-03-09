use xcap::Monitor;
use image::DynamicImage;
use std::io::Cursor;

pub fn capture_screen() -> Result<Vec<u8>, String> {
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    
    let primary = monitors.into_iter().find(|m| m.is_primary()).or_else(|| {
        Monitor::all().ok().and_then(|mut ms| if ms.is_empty() { None } else { Some(ms.remove(0)) })
    });

    if let Some(monitor) = primary {
        let image = monitor.capture_image().map_err(|e| e.to_string())?;
        
        let dyn_img = DynamicImage::ImageRgba8(image);
        let mut jpeg_data = Cursor::new(Vec::new());
        dyn_img.write_to(&mut jpeg_data, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
        
        return Ok(jpeg_data.into_inner());
    }
    
    Err("No monitor found".into())
}
