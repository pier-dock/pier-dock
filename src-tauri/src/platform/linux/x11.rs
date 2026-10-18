//! X11 via EWMH: lista e controla janelas em sessões X11 sem a extensão do GNOME, e reserva a
//! área da dock com `_NET_WM_STRUT_PARTIAL`.

use std::sync::OnceLock;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _, EventMask, ImageFormat, ImageOrder,
    Keycode, Keysym, PropMode, Visualtype, Window, KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

const XK_SUPER_L: Keysym = 0xffeb;

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        _NET_CLIENT_LIST,
        _NET_CLIENT_LIST_STACKING,
        _NET_ACTIVE_WINDOW,
        _NET_WM_NAME,
        _NET_WM_PID,
        _NET_WM_STATE,
        _NET_WM_STATE_HIDDEN,
        _NET_WM_STATE_SKIP_TASKBAR,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_NORMAL,
        _NET_WM_WINDOW_TYPE_DIALOG,
        _NET_WM_STRUT,
        _NET_WM_STRUT_PARTIAL,
        _NET_CLOSE_WINDOW,
        _NET_SHOWING_DESKTOP,
        WM_CHANGE_STATE,
        UTF8_STRING,
        _KDE_NET_WM_BLUR_BEHIND_REGION,
    }
}

pub struct X11 {
    conn: RustConnection,
    root: Window,
    root_height: u16,
    atoms: Atoms,
}

#[derive(Clone, Debug, PartialEq)]
pub struct X11Window {
    pub id: u32,
    pub title: String,
    pub pid: u32,
    /// Segunda parte do `WM_CLASS` (a "classe"), que costuma bater com o `StartupWMClass` do .desktop.
    pub wm_class: Option<String>,
    pub minimized: bool,
}

fn connect() -> Option<X11> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let screen = &conn.setup().roots[screen];
    let (root, root_height) = (screen.root, screen.height_in_pixels);
    let atoms = Atoms::new(&conn).ok()?.reply().ok()?;
    Some(X11 { conn, root, root_height, atoms })
}

/// Conexão compartilhada para consultas e comandos (o watcher de eventos abre a dele).
pub fn get() -> Option<&'static X11> {
    static X: OnceLock<Option<X11>> = OnceLock::new();
    X.get_or_init(connect).as_ref()
}

impl X11 {
    fn property32(&self, window: Window, property: u32, kind: impl Into<u32>) -> Vec<u32> {
        self.conn
            .get_property(false, window, property, kind.into(), 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().map(|v| v.collect()))
            .unwrap_or_default()
    }

    fn string_property(&self, window: Window, property: u32, kind: impl Into<u32>) -> Option<String> {
        let reply = self.conn.get_property(false, window, property, kind.into(), 0, 4096).ok()?.reply().ok()?;
        (!reply.value.is_empty()).then(|| String::from_utf8_lossy(&reply.value).into_owned())
    }

    fn title(&self, window: Window) -> String {
        self.string_property(window, self.atoms._NET_WM_NAME, self.atoms.UTF8_STRING)
            .or_else(|| self.string_property(window, AtomEnum::WM_NAME.into(), AtomEnum::STRING))
            .unwrap_or_default()
    }

    fn wm_class(&self, window: Window) -> Option<String> {
        let raw = self.string_property(window, AtomEnum::WM_CLASS.into(), AtomEnum::STRING)?;
        // "instância\0Classe\0"
        let mut parts = raw.split('\0').filter(|s| !s.is_empty());
        let instance = parts.next();
        parts.next().or(instance).map(str::to_string)
    }

    /// Mesmas regras de "aparece na barra de tarefas" que os painéis X11 usam.
    fn is_app_window(&self, window: Window) -> bool {
        let types = self.property32(window, self.atoms._NET_WM_WINDOW_TYPE, AtomEnum::ATOM);
        let normal = types.is_empty()
            || types.iter().any(|&t| t == self.atoms._NET_WM_WINDOW_TYPE_NORMAL || t == self.atoms._NET_WM_WINDOW_TYPE_DIALOG);
        let states = self.property32(window, self.atoms._NET_WM_STATE, AtomEnum::ATOM);
        normal && !states.contains(&self.atoms._NET_WM_STATE_SKIP_TASKBAR)
    }

    /// Janelas de app, a usada mais recentemente primeiro.
    pub fn windows(&self) -> Vec<X11Window> {
        let mut list = self.property32(self.root, self.atoms._NET_CLIENT_LIST_STACKING, AtomEnum::WINDOW);
        if list.is_empty() {
            list = self.property32(self.root, self.atoms._NET_CLIENT_LIST, AtomEnum::WINDOW);
        }
        list.reverse();
        list.into_iter()
            .filter(|&w| self.is_app_window(w))
            .map(|w| X11Window {
                id: w,
                title: self.title(w),
                pid: self.pid(w),
                wm_class: self.wm_class(w),
                minimized: self.is_minimized(w),
            })
            .filter(|w| !w.title.is_empty())
            .collect()
    }

    pub fn pid(&self, window: Window) -> u32 {
        self.property32(window, self.atoms._NET_WM_PID, AtomEnum::CARDINAL).first().copied().unwrap_or(0)
    }

    pub fn active_window(&self) -> Option<Window> {
        self.property32(self.root, self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW).first().copied().filter(|&w| w != 0)
    }

    pub fn is_minimized(&self, window: Window) -> bool {
        self.property32(window, self.atoms._NET_WM_STATE, AtomEnum::ATOM).contains(&self.atoms._NET_WM_STATE_HIDDEN)
    }

    fn send_to_root(&self, window: Window, kind: u32, data: [u32; 5]) {
        let event = ClientMessageEvent::new(32, window, kind, data);
        let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
        let _ = self.conn.send_event(false, self.root, mask, event);
        let _ = self.conn.flush();
    }

    /// Fonte 2 = "pager": o gerenciador de janelas obedece sem aplicar a prevenção de roubo de foco.
    pub fn activate(&self, window: Window) {
        self.send_to_root(window, self.atoms._NET_ACTIVE_WINDOW, [2, x11rb::CURRENT_TIME, 0, 0, 0]);
    }

    pub fn minimize(&self, window: Window) {
        const ICONIC_STATE: u32 = 3;
        self.send_to_root(window, self.atoms.WM_CHANGE_STATE, [ICONIC_STATE, 0, 0, 0, 0]);
    }

    pub fn close(&self, window: Window) {
        self.send_to_root(window, self.atoms._NET_CLOSE_WINDOW, [x11rb::CURRENT_TIME, 2, 0, 0, 0]);
    }

    /// O WM mantém `_NET_SHOWING_DESKTOP` em 1 enquanto as janelas estão escondidas.
    fn showing_desktop(&self) -> bool {
        self.property32(self.root, self.atoms._NET_SHOWING_DESKTOP, AtomEnum::CARDINAL).first().copied() == Some(1)
    }

    /// Entra ou sai do modo "mostrar área de trabalho". O EWMH define isso como um ClientMessage
    /// para a janela raiz; é o WM que esconde e devolve as janelas, conforme o pedido.
    pub fn toggle_showing_desktop(&self) {
        let showing = !self.showing_desktop();
        self.send_to_root(self.root, self.atoms._NET_SHOWING_DESKTOP, [showing as u32, 0, 0, 0, 0]);
    }

    pub fn move_resize(&self, window: Window, x: i32, y: i32, width: u32, height: u32) {
        let aux = ConfigureWindowAux::new().x(x).y(y).width(width).height(height);
        let _ = self.conn.configure_window(window, &aux);
        let _ = self.conn.flush();
    }

    /// Reserva `height` pixels acima de `bottom` (coordenadas da tela), entre `x` e `x + width`.
    /// `height == 0` remove a reserva.
    pub fn set_strut(&self, window: Window, x: i32, width: u32, bottom: i32, height: u32) {
        if height == 0 {
            let _ = self.conn.delete_property(window, self.atoms._NET_WM_STRUT_PARTIAL);
            let _ = self.conn.delete_property(window, self.atoms._NET_WM_STRUT);
        } else {
            // Struts são medidos a partir da borda da tela inteira (todas as telas juntas).
            let from_bottom = (self.root_height as i32 - bottom).max(0) as u32 + height;
            let (start, end) = (x.max(0) as u32, (x + width as i32 - 1).max(0) as u32);
            let partial = [0, 0, 0, from_bottom, 0, 0, 0, 0, 0, 0, start, end];
            let _ = self.conn.change_property32(PropMode::REPLACE, window, self.atoms._NET_WM_STRUT_PARTIAL, AtomEnum::CARDINAL, &partial);
            let _ = self.conn.change_property32(PropMode::REPLACE, window, self.atoms._NET_WM_STRUT, AtomEnum::CARDINAL, &partial[..4]);
        }
        let _ = self.conn.flush();
    }

    /// Toca a tecla Super pelo XTest. É o atalho do menu de apps no Cinnamon, MATE e Xfce, que não
    /// têm uma chamada pública para abri-lo.
    pub fn tap_super(&self) {
        let setup = self.conn.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let Some(mapping) = self.conn.get_keyboard_mapping(min, (max - min).saturating_add(1)).ok().and_then(|c| c.reply().ok())
        else {
            return;
        };
        let Some(keycode) = keycode_for(XK_SUPER_L, min, mapping.keysyms_per_keycode, &mapping.keysyms) else { return };
        for kind in [KEY_PRESS_EVENT, KEY_RELEASE_EVENT] {
            let _ = self.conn.xtest_fake_input(kind, keycode, x11rb::CURRENT_TIME, x11rb::NONE, 0, 0, 0);
        }
        let _ = self.conn.flush();
    }

    /// O KWin anuncia o efeito de blur pondo a propriedade na janela raiz.
    pub fn blur_supported(&self) -> bool {
        self.conn
            .get_property(false, self.root, self.atoms._KDE_NET_WM_BLUR_BEHIND_REGION, AtomEnum::ANY, 0, 0)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.type_ != u32::from(AtomEnum::NONE))
            .unwrap_or(false)
    }

    /// Regiões `[x, y, largura, altura]` (pixels da janela) que o KWin borra atrás da janela.
    /// Vazia remove o blur.
    pub fn set_blur_region(&self, window: Window, regions: &[[u32; 4]]) {
        if regions.is_empty() {
            let _ = self.conn.delete_property(window, self.atoms._KDE_NET_WM_BLUR_BEHIND_REGION);
        } else {
            let flat: Vec<u32> = regions.iter().flatten().copied().collect();
            let _ = self.conn.change_property32(
                PropMode::REPLACE,
                window,
                self.atoms._KDE_NET_WM_BLUR_BEHIND_REGION,
                AtomEnum::CARDINAL,
                &flat,
            );
        }
        let _ = self.conn.flush();
    }

    /// Conteúdo atual da janela como RGBA, para as prévias da dock.
    ///
    /// `XGetImage` na própria janela devolve o que o servidor X guarda dela — o mesmo caminho do
    /// `xwd` — e continua valendo com um compositor na frente, já que no X11 puro não há outra
    /// forma de ler os pixels de outro processo (o pixmap do `Composite` é do compositor).
    pub fn thumbnail(&self, window: Window) -> Option<image::RgbaImage> {
        if self.is_minimized(window) {
            return None;
        }
        let geom = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let (width, height) = (usize::from(geom.width), usize::from(geom.height));
        // Janelas de utilidade (1x1, 8x8) não viram prévia.
        if width < 48 || height < 48 {
            return None;
        }
        let setup = self.conn.setup();
        let format = setup.pixmap_formats.iter().find(|f| f.depth == geom.depth)?;
        let bytes_per_pixel = usize::from(format.bits_per_pixel).div_ceil(8);
        let scanline = usize::from(setup.bitmap_format_scanline_pad).max(8) / 8;
        let stride = (width * bytes_per_pixel).div_ceil(scanline) * scanline;
        let lsb = setup.image_byte_order == ImageOrder::LSB_FIRST;
        // As máscaras dos canais vêm do visual da janela: o formato do pixmap só diz o passo.
        let attrs = self.conn.get_window_attributes(window).ok()?.reply().ok()?;
        let (red_mask, green_mask, blue_mask) = {
            let visual = self.visual(attrs.visual)?;
            (visual.red_mask, visual.green_mask, visual.blue_mask)
        };

        let mut pixels = Vec::with_capacity(width * height * 4);
        let mut opaque = true;
        // Em faixas: uma resposta única de uma janela 4K passaria do tamanho máximo que o servidor
        // aceita devolver de uma vez.
        let band = (1 << 16) / width.max(1) + 1;
        let mut y = 0usize;
        while y < height {
            let rows = band.min(height - y);
            let reply = self
                .conn
                .get_image(ImageFormat::Z_PIXMAP, window, 0, y as i16, geom.width, rows as u16, !0)
                .ok()?
                .reply()
                .ok()?;
            for row in 0..rows {
                let start = row * stride;
                let line = reply.data.get(start..start + width * bytes_per_pixel)?;
                for px in line.chunks_exact(bytes_per_pixel) {
                    let value = pixel_value(px, lsb);
                    let alpha = if geom.depth == 32 { ((value >> 24) & 0xff) as u8 } else { 255 };
                    opaque &= alpha != 0;
                    pixels.extend_from_slice(&[
                        channel(value, red_mask),
                        channel(value, green_mask),
                        channel(value, blue_mask),
                        alpha,
                    ]);
                }
            }
            y += rows;
        }
        // Visual de 32 bits sem alfa nenhum no conteúdo: sem isto a prévia sairia vazia.
        if !opaque {
            for alpha in pixels.iter_mut().skip(3).step_by(4) {
                *alpha = 255;
            }
        }
        image::RgbaImage::from_raw(geom.width.into(), geom.height.into(), pixels)
    }

    /// Visual de uma janela pelo id, dentro do screen que a dock usa.
    fn visual(&self, id: u32) -> Option<&Visualtype> {
        let setup = self.conn.setup();
        let screen = setup.roots.iter().find(|root| root.root == self.root)?;
        screen.allowed_depths.iter().flat_map(|depth| depth.visuals.iter()).find(|v| v.visual_id == id)
    }
}

/// Valor do pixel na ordem de bytes do servidor (LSBFirst guarda o canal azul primeiro).
fn pixel_value(bytes: &[u8], lsb: bool) -> u32 {
    let mut value = 0u32;
    if lsb {
        for (i, byte) in bytes.iter().enumerate() {
            value |= u32::from(*byte) << (8 * i);
        }
    } else {
        for byte in bytes {
            value = (value << 8) | u32::from(*byte);
        }
    }
    value
}

/// Canal de 0 a 255 a partir da máscara do formato (visual verdadeiro costuma ser 5-6-5).
fn channel(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    (((value & mask) >> shift) * 255 / max) as u8
}

/// Keycode de `keysym` no mapa do teclado, que traz `per_keycode` keysyms para cada keycode a partir
/// de `min_keycode`.
fn keycode_for(keysym: Keysym, min_keycode: Keycode, per_keycode: u8, keysyms: &[Keysym]) -> Option<Keycode> {
    let index = keysyms.chunks(per_keycode.max(1) as usize).position(|syms| syms.contains(&keysym))?;
    min_keycode.checked_add(u8::try_from(index).ok()?)
}

/// Chama `on_change` quando a lista de janelas, o foco, um título ou um estado mudam. Usa uma
/// conexão própria, já que fica bloqueada esperando eventos.
pub fn watch(on_change: fn()) {
    std::thread::spawn(move || {
        let Some(x) = connect() else { return };
        let watch_props = ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE);
        let subscribe_clients = |x: &X11| {
            for w in x.property32(x.root, x.atoms._NET_CLIENT_LIST, AtomEnum::WINDOW) {
                let _ = x.conn.change_window_attributes(w, &watch_props);
            }
            let _ = x.conn.flush();
        };
        let _ = x.conn.change_window_attributes(x.root, &watch_props);
        subscribe_clients(&x);
        while let Ok(event) = x.conn.wait_for_event() {
            if let Event::PropertyNotify(e) = event {
                if e.window == x.root && e.atom == x.atoms._NET_CLIENT_LIST {
                    subscribe_clients(&x);
                }
                let relevant = [x.atoms._NET_CLIENT_LIST, x.atoms._NET_ACTIVE_WINDOW, x.atoms._NET_WM_NAME, x.atoms._NET_WM_STATE];
                if relevant.contains(&e.atom) || e.atom == u32::from(AtomEnum::WM_NAME) {
                    on_change();
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_super_keycode_in_mapping() {
        let mapping = [0x61, 0x41, XK_SUPER_L, 0, 0xff1b, 0];
        assert_eq!(keycode_for(XK_SUPER_L, 8, 2, &mapping), Some(9));
        assert_eq!(keycode_for(0x62, 8, 2, &mapping), None);
    }
}
