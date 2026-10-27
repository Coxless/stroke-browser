//! 外枠の UI（egui）。ページは egui の描く UI の下の層に、エンジンが描いたものを写して出す。

use std::sync::Arc;

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Key, LayerId, PaintCallback, TopBottomPanel};
use egui_glow::{CallbackFn, EguiGlow};
use egui_winit::EventResponse;
use stroke_engine::{Engine, LoadState, PixelRect};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::shortcuts;

/// UI での操作の結果、外枠にしてほしいこと
pub enum UiAction {
    /// URL 入力欄で Enter が押された
    Go(String),
    /// URL 入力欄を閉じた
    LocationClosed,
}

pub struct Ui {
    egui: EguiGlow,
    /// URL 入力欄の中身。`None` なら入力欄は閉じている
    location: Option<String>,
    /// 次の描画で URL 入力欄の中身を全部選ぶ
    select_location: bool,
    /// ページを表示する領域。単位は物理ピクセル、原点はウィンドウの左上
    view_rect: PixelRect,
    /// 状態表示に出すお知らせ（読み込みを止めた理由など）
    notice: Option<String>,
}

const LOCATION_ID: &str = "location";

impl Ui {
    pub fn new(event_loop: &ActiveEventLoop, gl: Arc<glow::Context>) -> Self {
        let egui = EguiGlow::new(event_loop, gl, None, None, false);
        egui.egui_ctx.options_mut(|options| {
            options.zoom_with_keyboard = false;
            options.fallback_theme = egui::Theme::Light;
        });
        // カーソルの点滅のための描き直しを止める。描き直しは、何かが変わったときだけにする
        egui.egui_ctx
            .all_styles_mut(|style| style.visuals.text_cursor.blink = false);
        add_japanese_font(&egui.egui_ctx);
        Self {
            egui,
            location: None,
            select_location: false,
            view_rect: PixelRect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            notice: None,
        }
    }

    pub fn set_notice(&mut self, notice: Option<String>) {
        self.notice = notice;
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui.egui_ctx
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.egui.on_window_event(window, event)
    }

    pub fn location_open(&self) -> bool {
        self.location.is_some()
    }

    /// URL 入力欄を開く。中身は今の URL にして、全部を選んでおく
    pub fn open_location(&mut self, current: String) {
        self.location = Some(current);
        self.select_location = true;
    }

    pub fn view_rect(&self) -> PixelRect {
        self.view_rect
    }

    /// UI を組み立て、ページを描く。ウィンドウに出すのは [`Self::paint`] で行う
    pub fn run(&mut self, window: &Window, engine: &dyn Engine) -> Vec<UiAction> {
        let mut actions = Vec::new();
        let Self {
            egui,
            location,
            select_location,
            view_rect,
            notice,
        } = self;
        egui.run(window, |ctx| {
            status_bar(ctx, engine, notice.as_deref());
            if let Some(text) = location {
                location_bar(ctx, text, select_location, &mut actions);
            }

            let rect = ctx.available_rect();
            let scale = ctx.pixels_per_point();
            *view_rect = PixelRect {
                x: (rect.min.x * scale).round() as i32,
                y: (rect.min.y * scale).round() as i32,
                width: (rect.width() * scale).round() as i32,
                height: (rect.height() * scale).round() as i32,
            };

            // UI の大きさに合わせてページの表示領域を変えてから、ページを描いて写す。
            // 順番を逆にすると、写す関数が大きさを変える前の（消された）バッファを指してしまう
            if view_rect.width > 0 && view_rect.height > 0 {
                engine.resize_view(PhysicalSize::new(
                    view_rect.width as u32,
                    view_rect.height as u32,
                ));
            }
            engine.paint();
            if let Some(composite) = engine.composite_fn() {
                ctx.layer_painter(LayerId::background()).add(PaintCallback {
                    rect,
                    callback: Arc::new(CallbackFn::new(move |info, painter| {
                        let clip = info.viewport_in_pixels();
                        composite(
                            painter.gl(),
                            PixelRect {
                                x: clip.left_px,
                                y: clip.from_bottom_px,
                                width: clip.width_px,
                                height: clip.height_px,
                            },
                        );
                    })),
                });
            }
        });
        // どちらの操作でも、入力欄は閉じる
        if !actions.is_empty() {
            self.location = None;
        }
        actions
    }

    pub fn paint(&mut self, window: &Window) {
        self.egui.paint(window);
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.egui.destroy();
    }
}

fn status_bar(ctx: &egui::Context, engine: &dyn Engine, notice: Option<&str>) {
    TopBottomPanel::bottom("status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let state = match engine.load_state() {
                LoadState::Started => "読み込み中",
                LoadState::HeadParsed => "表示中",
                LoadState::Complete => "完了",
            };
            ui.label(state);
            ui.separator();
            if let Some(notice) = notice {
                ui.colored_label(ui.visuals().warn_fg_color, notice);
                ui.separator();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(shortcuts::HELP);
                ui.separator();
                let url = engine.url().map(|url| url.to_string()).unwrap_or_default();
                ui.add(egui::Label::new(url).truncate());
            });
        });
    });
}

fn location_bar(
    ctx: &egui::Context,
    text: &mut String,
    select_all: &mut bool,
    actions: &mut Vec<UiAction>,
) {
    TopBottomPanel::top("location").show(ctx, |ui| {
        let id = egui::Id::new(LOCATION_ID);
        // 入力欄は開いている間ずっとキーを受け取る。閉じるのは Enter と Esc だけ。
        // 入力欄を置く前にフォーカスを渡しておく（置いた後だと、この描画の入力を受け取れない）
        ui.memory_mut(|memory| memory.request_focus(id));
        ui.add_sized(
            [ui.available_width(), 24.0],
            egui::TextEdit::singleline(text)
                .id(id)
                .hint_text("URL か検索語を入力（Enter で開く、Esc で閉じる）"),
        );
        if std::mem::take(select_all)
            && let Some(mut state) = TextEditState::load(ui.ctx(), id)
        {
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                CCursor::new(text.chars().count()),
            )));
            state.store(ui.ctx(), id);
        }
        let (enter, escape) = ui.input(|input| {
            (
                input.key_pressed(Key::Enter),
                input.key_pressed(Key::Escape),
            )
        });
        if enter {
            actions.push(UiAction::Go(text.clone()));
        } else if escape {
            actions.push(UiAction::LocationClosed);
        }
    });
}

/// egui の既定のフォントには日本語の字がないので、Noto CJK を足す
fn add_japanese_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    ];
    let Some(bytes) = CANDIDATES.iter().find_map(|path| std::fs::read(path).ok()) else {
        log::warn!("日本語のフォントが見つからない。URL 入力欄で日本語が出ない");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    // NotoSansCJK-Regular.ttc の 0 番目が日本語（JP）
    let mut data = egui::FontData::from_owned(bytes);
    data.index = 0;
    fonts.font_data.insert("noto-cjk".into(), Arc::new(data));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("noto-cjk".into());
    }
    ctx.set_fonts(fonts);
}
