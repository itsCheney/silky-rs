//! Desktop player: a window that shows a [`Session`] and forwards mouse and
//! keyboard input to it.
//!
//! Frames come from the software renderer and are scaled to the window in
//! software, letterboxed to 16:9.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use clap::Parser;
use silky_format::game::Game;
use silky_runtime::render::Canvas;
use silky_runtime::text::TextRenderer;
use silky_runtime::{Config, Language, Session, State, SCREEN_HEIGHT, SCREEN_WIDTH};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Game folder (with Patch.xml or the archives).
    game: PathBuf,
    /// Text language: 0 English, 1 Simplified Chinese, 2 Traditional Chinese.
    #[arg(long, default_value_t = 0)]
    lang: usize,
    /// Font files for the message text, in fallback order. Without one,
    /// the system fonts the game itself uses are tried.
    #[arg(long)]
    font: Vec<PathBuf>,
    /// Script to start with.
    #[arg(long, default_value = "start.mes")]
    start: String,
}

/// Fonts the game names in its settings, by language, then common
/// fallbacks on other systems.
fn default_fonts(lang: Language) -> Vec<PathBuf> {
    let windows = std::env::var_os("WINDIR")
        .map(|w| PathBuf::from(w).join("Fonts"))
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));
    let mut names: Vec<PathBuf> = match lang {
        Language::English => vec![windows.join("arial.ttf")],
        Language::SimplifiedChinese => vec![windows.join("msyh.ttc")],
        Language::TraditionalChinese => vec![windows.join("msjh.ttc")],
    };
    names.extend(
        [
            r"C:\Windows\Fonts\msyh.ttc",
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
        .map(PathBuf::from),
    );
    names
}

fn load_fonts(cli: &Cli, lang: Language) -> Result<Option<TextRenderer>> {
    let files = if cli.font.is_empty() {
        let found: Vec<_> = default_fonts(lang)
            .into_iter()
            .filter(|p| p.is_file())
            .take(2)
            .collect();
        if found.is_empty() {
            eprintln!("no font found: text is not shown (pass --font)");
            return Ok(None);
        }
        found
    } else {
        cli.font.clone()
    };
    let data = files
        .iter()
        .map(|f| std::fs::read(f).with_context(|| format!("reading {}", f.display())))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(TextRenderer::new(&data)?))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let language = Language::from_section(cli.lang).context("--lang must be 0, 1 or 2")?;
    let game = Game::open(&cli.game)
        .with_context(|| format!("cannot open game folder {}", cli.game.display()))?;
    let text = load_fonts(&cli, language)?;
    let config = Config {
        language,
        ..Config::default()
    };
    let session = Session::new(game, config, text, &cli.start)?;
    let event_loop = EventLoop::new()?;
    let mut app = App {
        session,
        view: None,
        last_tick: Instant::now(),
        cursor: (0, 0),
        error: None,
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct View {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    /// Where the game picture sits in the window: x, y, width, height.
    rect: (u32, u32, u32, u32),
}

struct App {
    session: Session,
    view: Option<View>,
    last_tick: Instant,
    /// Cursor in game pixels.
    cursor: (i32, i32),
    error: Option<anyhow::Error>,
}

const FRAME: Duration = Duration::from_millis(16);

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, e: impl Into<anyhow::Error>) {
        self.error = Some(e.into());
        event_loop.exit();
    }

    fn to_game(&self, p: PhysicalPosition<f64>) -> (i32, i32) {
        let Some(view) = &self.view else {
            return (0, 0);
        };
        let (x, y, w, h) = view.rect;
        let gx = (p.x - x as f64) * SCREEN_WIDTH as f64 / w.max(1) as f64;
        let gy = (p.y - y as f64) * SCREEN_HEIGHT as f64 / h.max(1) as f64;
        (gx.floor() as i32, gy.floor() as i32)
    }

    fn redraw(&mut self) -> Result<()> {
        let Some(view) = &mut self.view else {
            return Ok(());
        };
        let size = view.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Ok(());
        };
        view.surface
            .resize(w, h)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        view.rect = letterbox(size.width, size.height);
        let frame = self.session.frame()?;
        let mut buf = view
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        blit(frame, &mut buf, size.width, size.height, view.rect);
        buf.present().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    fn after_input(&mut self, event_loop: &ActiveEventLoop, r: silky_runtime::Result<()>) {
        if let Err(e) = r {
            return self.fail(event_loop, e);
        }
        if self.session.state() == State::Ended {
            event_loop.exit();
        }
        if let Some(v) = &self.view {
            v.window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Silky Player")
            .with_inner_size(LogicalSize::new(SCREEN_WIDTH, SCREEN_HEIGHT))
            .with_min_inner_size(LogicalSize::new(SCREEN_WIDTH / 4, SCREEN_HEIGHT / 4));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => return self.fail(event_loop, e),
        };
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
        match surface {
            Ok(surface) => {
                let size = window.inner_size();
                self.view = Some(View {
                    window,
                    surface,
                    rect: letterbox(size.width, size.height),
                });
            }
            Err(e) => self.fail(event_loop, anyhow::anyhow!("{e}")),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, e);
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(v) = &self.view {
                    v.window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self.to_game(position);
                let (x, y) = self.cursor;
                self.session.pointer_move(x, y);
                self.after_input(event_loop, Ok(()));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let (x, y) = self.cursor;
                let r = match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.session.pointer_down(x, y);
                        Ok(())
                    }
                    (MouseButton::Left, ElementState::Released) => self.session.pointer_up(x, y),
                    (MouseButton::Right, ElementState::Released) => {
                        self.session.secondary_click();
                        Ok(())
                    }
                    _ => Ok(()),
                };
                self.after_input(event_loop, r);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let r = match event.logical_key {
                    Key::Named(NamedKey::Enter | NamedKey::Space) => self.session.proceed(),
                    _ => Ok(()),
                };
                self.after_input(event_loop, r);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let ms = now.duration_since(self.last_tick).as_millis() as u64;
        if ms >= FRAME.as_millis() as u64 {
            self.last_tick = now;
            match self.session.tick(ms) {
                Ok(true) => self.after_input(event_loop, Ok(())),
                Ok(false) => {}
                Err(e) => return self.fail(event_loop, e),
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_tick + FRAME));
    }
}

/// The largest 16:9 rectangle centred in a window.
fn letterbox(w: u32, h: u32) -> (u32, u32, u32, u32) {
    let (sw, sh) = (SCREEN_WIDTH as u64, SCREEN_HEIGHT as u64);
    let (fw, fh) = if w as u64 * sh > h as u64 * sw {
        ((h as u64 * sw / sh) as u32, h)
    } else {
        (w, (w as u64 * sh / sw) as u32)
    };
    ((w - fw) / 2, (h - fh) / 2, fw.max(1), fh.max(1))
}

/// Scales the frame into the window buffer (0RGB), bilinear, with black
/// bars around it.
fn blit(frame: &Canvas, buf: &mut [u32], w: u32, h: u32, rect: (u32, u32, u32, u32)) {
    let (rx, ry, rw, rh) = rect;
    buf.fill(0);
    let fw = frame.width as usize;
    let px = |x: usize, y: usize| -> [f32; 3] {
        let i = (y * fw + x) * 4;
        let p = &frame.rgba[i..i + 3];
        [p[0] as f32, p[1] as f32, p[2] as f32]
    };
    let sx = frame.width as f32 / rw as f32;
    let sy = frame.height as f32 / rh as f32;
    for y in 0..rh.min(h - ry) {
        let fy = ((y as f32 + 0.5) * sy - 0.5).max(0.0);
        let y0 = (fy as usize).min(frame.height as usize - 1);
        let y1 = (y0 + 1).min(frame.height as usize - 1);
        let ty = fy - y0 as f32;
        let row = &mut buf[((ry + y) * w + rx) as usize..][..rw.min(w - rx) as usize];
        for (x, out) in row.iter_mut().enumerate() {
            let fx = ((x as f32 + 0.5) * sx - 0.5).max(0.0);
            let x0 = (fx as usize).min(fw - 1);
            let x1 = (x0 + 1).min(fw - 1);
            let tx = fx - x0 as f32;
            let (a, b, c, d) = (px(x0, y0), px(x1, y0), px(x0, y1), px(x1, y1));
            let mut rgb = [0u32; 3];
            for k in 0..3 {
                let top = a[k] + (b[k] - a[k]) * tx;
                let bottom = c[k] + (d[k] - c[k]) * tx;
                rgb[k] = (top + (bottom - top) * ty).round() as u32;
            }
            *out = (rgb[0] << 16) | (rgb[1] << 8) | rgb[2];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::letterbox;

    #[test]
    fn letterbox_fits_16_9() {
        assert_eq!(letterbox(1280, 720), (0, 0, 1280, 720));
        assert_eq!(letterbox(1920, 1200), (0, 60, 1920, 1080));
        assert_eq!(letterbox(1000, 720), (0, 79, 1000, 562));
    }
}
