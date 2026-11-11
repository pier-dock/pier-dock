//! Ícones no macOS: o mesmo que o Finder mostra (NSWorkspace), renderizado em PNG 256×256.

use std::path::Path;

use objc2::rc::autoreleasepool;
use objc2::AllocAnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

const SIZE: f64 = 256.0;

/// A chave é um caminho (app, pasta, arquivo) ou um bundle id.
fn resolve(key: &str) -> Option<String> {
    if key.starts_with('/') {
        return Path::new(key).exists().then(|| key.to_string());
    }
    let url = NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString::from_str(key))?;
    url.path().map(|p| p.to_string())
}

pub fn extract(key: &str) -> Result<(Vec<u8>, &'static str), String> {
    autoreleasepool(|_| render(key))
}

fn render(key: &str) -> Result<(Vec<u8>, &'static str), String> {
    let path = resolve(key).ok_or_else(|| format!("sem ícone para {key}"))?;
    let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(&path));
    let size = NSSize::new(SIZE, SIZE);
    image.setSize(size);
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    let cg_image = unsafe { image.CGImageForProposedRect_context_hints(&mut rect, None, None) }
        .ok_or(crate::i18n::tr("falha ao renderizar o ícone", "couldn't render the icon"))?;
    let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg_image);
    let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }
        .ok_or(crate::i18n::tr("falha ao gerar o PNG do ícone", "couldn't encode the icon PNG"))?;
    Ok((png.to_vec(), "image/png"))
}
