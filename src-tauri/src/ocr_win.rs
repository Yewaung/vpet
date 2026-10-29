//! Offline text recognition with the OCR built into Windows
//! (Windows.Media.Ocr). Used only on frames of a screen the user explicitly
//! chose to share; the image and the text are never stored.

/// Reads the text in an RGBA image. Keep images at or below
/// `OcrEngine::MaxImageDimension` (2600 px); callers downscale first.
#[cfg(windows)]
pub fn recognize(rgba: &[u8], width: u32, height: u32) -> Result<String, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        return Err("image size doesn't match its data".into());
    }
    // SAFETY: initialising WinRT on this worker thread; "already initialised" is fine.
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    let err = |e: windows::core::Error| e.message().to_string();
    let mut bgra = rgba.to_vec();
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    let writer = DataWriter::new().map_err(err)?;
    writer.WriteBytes(&bgra).map_err(err)?;
    let buffer = writer.DetachBuffer().map_err(err)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width as i32, height as i32).map_err(err)?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|_| "no OCR language is installed on Windows".to_string())?;
    let result = engine.RecognizeAsync(&bitmap).map_err(err)?.join().map_err(err)?;
    Ok(result.Text().map_err(err)?.to_string())
}

#[cfg(not(windows))]
pub fn recognize(_rgba: &[u8], _width: u32, _height: u32) -> Result<String, String> {
    Err("text recognition needs Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// Draws words with GDI into a bitmap and reads them back.
    #[test]
    fn reads_text_from_an_image() {
        use windows_sys::Win32::Graphics::Gdi::*;
        let (w, h) = (640i32, 120i32);
        let text: Vec<u16> = "the pizza was delicious".encode_utf16().collect();
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        // SAFETY: plain GDI drawing into a memory DC we own and release.
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = w;
            info.bmiHeader.biHeight = -h;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            let mut bits = std::ptr::null_mut();
            let bmp = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
            let old = SelectObject(dc, bmp);
            let brush = CreateSolidBrush(0x00FFFFFF);
            let rect = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: w, bottom: h };
            FillRect(dc, &rect, brush);
            let font = CreateFontW(48, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 0, 0, std::ptr::null());
            SelectObject(dc, font);
            TextOutW(dc, 10, 30, text.as_ptr(), text.len() as i32);
            std::ptr::copy_nonoverlapping(bits as *const u8, pixels.as_mut_ptr(), pixels.len());
            SelectObject(dc, old);
            DeleteObject(font);
            DeleteObject(brush);
            DeleteObject(bmp);
            DeleteDC(dc);
        }
        // GDI gives BGRA; recognize() wants RGBA.
        for px in pixels.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }
        let read = match recognize(&pixels, w as u32, h as u32) {
            Ok(text) => text.to_lowercase(),
            // CI machines may have no OCR language pack installed.
            Err(e) if e.contains("no OCR language") => return eprintln!("skipped: {e}"),
            Err(e) => panic!("{e}"),
        };
        assert!(read.contains("pizza"), "read: {read:?}");
    }
}
