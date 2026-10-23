//! Stroke browser の実行ファイル。ウィンドウ、外枠の UI、エンジンをつなぐ。
//!
//!     stroke [URL]
//!
//! キーはすべて外枠が先に受け取り、外枠の命令でなければページに渡す。

mod location;
mod shortcuts;
mod ui;

use stroke_engine::{Engine, EngineEvent, PixelRect, ServoEngine, Url};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::shortcuts::Command;
use crate::ui::{Ui, UiAction};

const START_URL: &str = "https://servo.org/";

fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("TLS の暗号の実装を入れられない");

    let url = std::env::args()
        .nth(1)
        .and_then(|input| location::resolve(&input))
        .unwrap_or_else(|| Url::parse(START_URL).expect("START_URL は正しい URL"));

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("イベントループを作れない");
    let mut app = App {
        proxy: event_loop.create_proxy(),
        start_url: Some(url),
        running: None,
    };
    event_loop
        .run_app(&mut app)
        .expect("イベントループが異常終了した");
}

/// ウィンドウのイベント以外で、イベントループを起こす理由
#[derive(Debug)]
enum AppEvent {
    /// エンジンに仕事がある
    Wake,
    /// egui が描き直しを求めている
    Repaint,
}

struct App {
    proxy: EventLoopProxy<AppEvent>,
    start_url: Option<Url>,
    running: Option<Running>,
}

/// フィールドは書いた順に片付けられる。UI（egui の OpenGL の資源）を、エンジンの持つ
/// OpenGL の文脈とウィンドウより先に片付けるため、この順に並べている
struct Running {
    ui: Ui,
    engine: Box<dyn Engine>,
    window: Window,
    modifiers: ModifiersState,
    /// ページの入力欄が IME を求めているときの、入力欄の位置（ページの表示領域が原点）
    page_ime_area: Option<PixelRect>,
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(url) = self.start_url.take() else {
            return;
        };
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Stroke")
                    .with_inner_size(PhysicalSize::new(1280, 900)),
            )
            .expect("ウィンドウを作れない");

        let proxy = self.proxy.clone();
        let engine = ServoEngine::new(
            &window,
            move || {
                let _ = proxy.send_event(AppEvent::Wake);
            },
            url,
        )
        .unwrap_or_else(|error| panic!("エンジンを起動できない: {error}"));

        engine.make_current();
        let ui = Ui::new(event_loop, engine.gl());
        let proxy = self.proxy.clone();
        ui.egui_ctx().set_request_repaint_callback(move |_| {
            let _ = proxy.send_event(AppEvent::Repaint);
        });

        self.running = Some(Running {
            ui,
            engine: Box::new(engine),
            window,
            modifiers: ModifiersState::empty(),
            page_ime_area: None,
        });
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        match event {
            AppEvent::Wake => running.spin(),
            AppEvent::Repaint => running.window.request_redraw(),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        running.spin();
        if running.handle_window_event(event) == Flow::Exit {
            event_loop.exit();
        }
    }
}

#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Exit,
}

impl Running {
    /// エンジンの仕事を進め、溜まった通知を処理する
    fn spin(&mut self) {
        self.engine.spin();
        for event in self.engine.take_events() {
            if event != EngineEvent::NewFrameReady {
                log::debug!("エンジン {event:?}");
            }
            match event {
                EngineEvent::NewFrameReady
                | EngineEvent::UrlChanged(_)
                | EngineEvent::LoadStateChanged(_) => self.window.request_redraw(),
                EngineEvent::TitleChanged(title) => {
                    let title = match title {
                        Some(title) if !title.is_empty() => format!("{title} — Stroke"),
                        _ => "Stroke".into(),
                    };
                    self.window.set_title(&title);
                }
                EngineEvent::InputMethodShown { area } => {
                    self.page_ime_area = Some(area);
                    self.apply_page_ime();
                }
                EngineEvent::InputMethodHidden => {
                    self.page_ime_area = None;
                    self.apply_page_ime();
                }
            }
        }
    }

    fn handle_window_event(&mut self, event: WindowEvent) -> Flow {
        match &event {
            WindowEvent::CloseRequested => return Flow::Exit,
            WindowEvent::RedrawRequested => {
                self.redraw();
                return Flow::Continue;
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Resized(size) => {
                self.engine.resize_window(*size);
                self.window.request_redraw();
            }
            // URL 入力欄が閉じているときは、キーと IME はページのもの。egui には渡さない
            WindowEvent::KeyboardInput { event: key, .. } if !self.ui.location_open() => {
                if key.state == ElementState::Pressed
                    && let Some(command) = shortcuts::command_for(&key.logical_key, self.modifiers)
                {
                    log::debug!(
                        "キー {:?} {:?} → {command:?}",
                        self.modifiers,
                        key.logical_key
                    );
                    return self.run_command(command);
                }
                log::debug!(
                    "キー {:?} {:?} {:?} → ページ",
                    self.modifiers,
                    key.logical_key,
                    key.state
                );
                self.engine.send_key(key, self.modifiers);
                return Flow::Continue;
            }
            WindowEvent::Ime(ime) if !self.ui.location_open() => {
                self.engine.send_ime(ime);
                return Flow::Continue;
            }
            _ => {}
        }
        if self.ui.on_window_event(&self.window, &event).repaint {
            self.window.request_redraw();
        }
        Flow::Continue
    }

    fn run_command(&mut self, command: Command) -> Flow {
        match command {
            Command::OpenLocation => {
                let current = self
                    .engine
                    .url()
                    .map(|url| url.to_string())
                    .unwrap_or_default();
                self.ui.open_location(current);
                // egui はウィンドウのフォーカスを WindowEvent::Focused で知るが、ウィンドウ
                // マネージャーによっては届かない（Xvfb など）。届かないと入力欄が文字を受け付けない。
                // キーがこのウィンドウに来た以上フォーカスはあるので、ここで知らせておく
                let _ = self
                    .ui
                    .on_window_event(&self.window, &WindowEvent::Focused(true));
                self.window.request_redraw();
            }
            Command::Back => self.engine.go_back(),
            Command::Forward => self.engine.go_forward(),
            Command::Reload => self.engine.reload(),
            Command::Quit => return Flow::Exit,
        }
        Flow::Continue
    }

    fn redraw(&mut self) {
        self.engine.make_current();
        let actions = self.ui.run(&self.window, self.engine.as_ref());
        self.engine.begin_frame();
        self.ui.paint(&self.window);
        self.engine.present();

        for action in actions {
            if let UiAction::Go(input) = action
                && let Some(url) = location::resolve(&input)
            {
                self.engine.load(url);
            }
            // 入力欄を閉じると egui が IME を切るので、ページの入力欄の分を戻す
            self.apply_page_ime();
            self.window.request_redraw();
        }
    }

    /// ページの入力欄の状態に合わせて、IME を有効・無効にする
    fn apply_page_ime(&self) {
        if self.ui.location_open() {
            return;
        }
        let Some(area) = self.page_ime_area else {
            self.window.set_ime_allowed(false);
            return;
        };
        let view = self.ui.view_rect();
        self.window.set_ime_allowed(true);
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(view.x + area.x, view.y + area.y),
            PhysicalSize::new(area.width.max(1), area.height.max(1)),
        );
    }
}
