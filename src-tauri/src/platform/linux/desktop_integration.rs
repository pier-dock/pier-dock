//! Cadastro no menu de aplicativos quando o Pier roda como AppImage.
//!
//! O runtime do AppImage não instala nada: sem isso, quem baixa o AppImage abre a dock, mas ela
//! não aparece no menu do KDE, do GNOME nem de um launcher como o Rofi — e cada pessoa acaba
//! escrevendo a entrada à mão. Na primeira execução o Pier copia a entrada e os ícones que já vêm
//! dentro do próprio AppImage para `~/.local/share`, e nada mais é preciso.
//!
//! Um `.deb` (ou um pacote da distro) já traz a entrada pronta: o arquivo existe e nada é
//! reescrito. Sem as variáveis `APPIMAGE`/`APPDIR` — `cargo run`, `npm run tauri dev` — não faz nada.

use std::fs;
use std::path::{Path, PathBuf};

/// Usada quando a entrada empacotada não pode ser lida, para o cadastro não depender dela.
const FALLBACK: &str = "[Desktop Entry]\n\
Type=Application\n\
Name=Pier\n\
GenericName=Dock\n\
Comment=A customizable dock with pinned apps, magnification and live widgets\n\
Exec={exec}\n\
Icon=pier\n\
Terminal=false\n\
Categories=Utility;\n\
Keywords=dock;launcher;taskbar;panel;\n\
StartupWMClass=pier\n";

/// Registra a entrada de menu do AppImage, uma vez só. Qualquer falha é silenciosa: a dock abre
/// do mesmo jeito, ela só não aparece no menu.
pub fn install_if_needed() {
    let Some(appimage) = std::env::var_os("APPIMAGE") else { return };
    let Some(appdir) = std::env::var_os("APPDIR").map(PathBuf::from) else { return };
    let Some(data) = dirs::data_dir() else { return };

    let entry = data.join("applications/pier.desktop");
    if entry.exists() {
        return;
    }

    let exec = appimage.to_string_lossy();
    let content = match fs::read_to_string(appdir.join("usr/share/applications/pier.desktop")) {
        Ok(bundled) => with_exec(&bundled, &exec),
        Err(_) => FALLBACK.replace("{exec}", &exec),
    };

    let Some(parent) = entry.parent() else { return };
    if fs::create_dir_all(parent).is_err() || fs::write(&entry, content).is_err() {
        return;
    }
    copy_icons(&appdir, &data);
}

/// A entrada empacotada chama só `pier`, que não existe no PATH de quem usa o AppImage: o que vale
/// é o caminho do arquivo, entre aspas porque ele pode ter espaços.
fn with_exec(entry: &str, exec: &str) -> String {
    let mut out = String::with_capacity(entry.len() + exec.len() + 8);
    let mut replaced = false;
    for line in entry.lines() {
        if !replaced && line.starts_with("Exec=") {
            replaced = true;
            out.push_str(&format!("Exec=\"{exec}\"\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        out.push_str(&format!("Exec=\"{exec}\"\n"));
    }
    out
}

/// Os ícones que já vêm no AppImage (`usr/share/icons/hicolor/<tamanho>/apps/pier.png`) vão para o
/// tema do usuário. Um arquivo que já existe não é sobrescrito, para não brigar com uma instalação
/// do `.deb` nem com um tema editado à mão.
fn copy_icons(appdir: &Path, data: &Path) {
    let Ok(sizes) = fs::read_dir(appdir.join("usr/share/icons/hicolor")) else { return };
    for size in sizes.flatten() {
        let source = size.path().join("apps/pier.png");
        if !source.is_file() {
            continue;
        }
        let target = data.join("icons/hicolor").join(size.file_name()).join("apps/pier.png");
        if target.exists() {
            continue;
        }
        let Some(parent) = target.parent() else { continue };
        if fs::create_dir_all(parent).is_ok() {
            let _ = fs::copy(&source, &target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::with_exec;

    const BUNDLED: &str = "[Desktop Entry]\nType=Application\nName=Pier\nExec=pier\nIcon=pier\nTerminal=false\n";

    #[test]
    fn troca_o_exec_pelo_caminho_do_appimage() {
        let entry = with_exec(BUNDLED, "/home/luiz/Apps/Pier 0.5.0.AppImage");
        assert!(entry.contains("Exec=\"/home/luiz/Apps/Pier 0.5.0.AppImage\"\n"));
        assert!(!entry.contains("Exec=pier\n"));
        assert!(entry.contains("Icon=pier\n"));
        assert_eq!(entry.lines().count(), BUNDLED.lines().count());
    }

    #[test]
    fn acrescenta_o_exec_quando_falta() {
        let entry = with_exec("[Desktop Entry]\nName=Pier\n", "/opt/pier.AppImage");
        assert!(entry.ends_with("Exec=\"/opt/pier.AppImage\"\n"));
    }

    #[test]
    fn nao_mexe_no_resto_da_entrada() {
        assert_eq!(with_exec(BUNDLED, "/p.AppImage").matches("Name=Pier").count(), 1);
    }
}
