//! Ícones em alta resolução de apps, arquivos e pastas, devolvidos como data URL e cacheados em
//! disco. Cada sistema tem um backend com `extract(key) -> (bytes, mime)`; a chave é o que o
//! frontend sabe do app (caminho, `shell:AppsFolder\…`, desktop id, bundle .app).

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use std::hash::{Hash, Hasher};

use base64::Engine;

use crate::util::{data_dir, reject_network_path, ErrStr};

/// Formatos que o cache guarda, pela extensão do arquivo.
const FORMATS: [(&str, &str); 2] = [("png", "image/png"), ("svg", "image/svg+xml")];

fn cache_path(key: &str, ext: &str) -> std::path::PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.to_lowercase().hash(&mut hasher);
    let dir = data_dir().join("icons");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{:016x}.{ext}", hasher.finish()))
}

/// Lado, em px, da versão para listas. Cobre ícones de até 48 px lógicos em tela retina.
const SMALL_SIZE: u32 = 96;

fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn full_icon(key: &str) -> Result<(Vec<u8>, &'static str), String> {
    let cached = FORMATS.iter().find_map(|(ext, mime)| std::fs::read(cache_path(key, ext)).ok().map(|b| (b, *mime)));
    Ok(match cached {
        Some(hit) => hit,
        None => {
            let (bytes, mime) = backend::extract(key)?;
            if let Some((ext, _)) = FORMATS.iter().find(|(_, m)| *m == mime) {
                let _ = std::fs::write(cache_path(key, ext), &bytes);
            }
            (bytes, mime)
        }
    })
}

/// Reduz o PNG para `SMALL_SIZE`. O redimensionamento mistura os canais, então com alfa reto os
/// pixels transparentes (RGB 0) escureceriam a borda: a conta é feita com alfa pré-multiplicado.
fn shrink(png: &[u8]) -> Option<Vec<u8>> {
    let mut image = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.into_rgba8();
    let longest = image.width().max(image.height());
    if longest <= SMALL_SIZE {
        return None;
    }
    for pixel in image.pixels_mut() {
        let alpha = pixel[3] as u32;
        for channel in &mut pixel.0[..3] {
            *channel = ((*channel as u32 * alpha + 127) / 255) as u8;
        }
    }
    let scale = SMALL_SIZE as f32 / longest as f32;
    let width = ((image.width() as f32 * scale).round() as u32).max(1);
    let height = ((image.height() as f32 * scale).round() as u32).max(1);
    let mut small = image::imageops::resize(&image, width, height, image::imageops::FilterType::Lanczos3);
    for pixel in small.pixels_mut() {
        let alpha = pixel[3] as u32;
        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel.0[..3] {
                *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    small.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
pub fn icon_data_url(key: &str) -> Result<String, String> {
    icon_data_url_sized(key, false)
}

/// `small` devolve a versão para listas (ver `SMALL_SIZE`), cacheada à parte em disco. SVG e ícones
/// que já cabem no tamanho saem como estão.
pub fn icon_data_url_sized(key: &str, small: bool) -> Result<String, String> {
    reject_network_path(key)?;
    if small {
        if let Ok(bytes) = std::fs::read(cache_path(key, "s.png")) {
            return Ok(data_url("image/png", &bytes));
        }
    }
    let (bytes, mime) = full_icon(key)?;
    if small && mime == "image/png" {
        if let Some(shrunk) = shrink(&bytes) {
            let _ = std::fs::write(cache_path(key, "s.png"), &shrunk);
            return Ok(data_url(mime, &shrunk));
        }
    }
    Ok(data_url(mime, &bytes))
}

#[tauri::command]
pub async fn get_icon(key: String, small: Option<bool>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || icon_data_url_sized(&key, small.unwrap_or(false))).await.err_str()?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(image: &image::RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn shrinks_to_the_small_size_without_darkening_transparent_edges() {
        let source = image::RgbaImage::from_fn(256, 256, |x, y| {
            if (64..192).contains(&x) && (64..192).contains(&y) {
                image::Rgba([255, 255, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        });
        let small = image::load_from_memory(&shrink(&png(&source)).unwrap()).unwrap().into_rgba8();
        assert_eq!((small.width(), small.height()), (SMALL_SIZE, SMALL_SIZE));
        for pixel in small.pixels().filter(|p| p[3] > 0) {
            assert!(pixel[0] >= 250, "edge pixel went dark: {pixel:?}");
        }
    }

    #[test]
    fn leaves_icons_that_already_fit() {
        assert!(shrink(&png(&image::RgbaImage::new(SMALL_SIZE, SMALL_SIZE))).is_none());
    }
}
