//! [`Engine`] の Servo 実装。Servo の型はこのファイルと `keys.rs` の外に出さない。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use euclid::Scale;
use servo::{
    CompositionEvent, CompositionState, EmbedderControl, EmbedderControlId, EventLoopWaker,
    ImeEvent, InputEvent, JSValue, LoadStatus, OffscreenRenderingContext, RenderingContext, Servo,
    ServoBuilder, WebView, WebViewBuilder, WebViewDelegate, WindowRenderingContext,
};
use url::Url;
use winit::dpi::PhysicalSize;
use winit::event::{Ime, KeyEvent};
use winit::keyboard::ModifiersState;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::{CompositeFn, Engine, EngineEvent, LoadState, PixelRect, ScriptResult};

pub struct ServoEngine {
    servo: Servo,
    window_context: Rc<WindowRenderingContext>,
    /// ページはここに描き、外枠が UI と合わせてウィンドウに写す
    view_context: Rc<OffscreenRenderingContext>,
    webview: WebView,
    delegate: Rc<Delegate>,
}

impl ServoEngine {
    /// `window` に描くエンジンを作り、`url` を開く。`wake` はエンジンが別のスレッドから
    /// 外枠のイベントループを起こすときに呼ばれる。
    pub fn new(
        window: &Window,
        wake: impl Fn() + Send + Sync + 'static,
        url: Url,
    ) -> Result<Self, String> {
        let display_handle = window.display_handle().map_err(|e| e.to_string())?;
        let window_handle = window.window_handle().map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let window_context = Rc::new(
            WindowRenderingContext::new(display_handle, window_handle, size)
                .map_err(|e| format!("OpenGL の文脈を作れない: {e:?}"))?,
        );
        window_context
            .make_current()
            .map_err(|e| format!("OpenGL の文脈を使えない: {e:?}"))?;
        let view_context = Rc::new(window_context.offscreen_context(size));

        let servo = ServoBuilder::default()
            .event_loop_waker(Box::new(Waker(Arc::new(wake))))
            .build();
        servo.setup_logging();

        let delegate = Rc::new(Delegate::default());
        let webview = WebViewBuilder::new(&servo, view_context.clone())
            .url(url)
            .hidpi_scale_factor(Scale::new(window.scale_factor() as f32))
            .delegate(delegate.clone())
            .build();
        // キーを受け取れるように、最初から WebView にフォーカスを与えておく
        webview.focus();

        Ok(Self {
            servo,
            window_context,
            view_context,
            webview,
            delegate,
        })
    }
}

impl Engine for ServoEngine {
    fn load(&self, url: Url) {
        self.webview.load(url);
    }

    fn go_back(&self) {
        if self.webview.can_go_back() {
            self.webview.go_back(1);
        }
    }

    fn go_forward(&self) {
        if self.webview.can_go_forward() {
            self.webview.go_forward(1);
        }
    }

    fn reload(&self) {
        self.webview.reload();
    }

    fn send_key(&self, event: &KeyEvent, modifiers: ModifiersState) {
        let event = crate::keys::keyboard_event(event, modifiers);
        self.webview.notify_input_event(InputEvent::Keyboard(event));
    }

    fn send_ime(&self, event: &Ime) {
        let composition = |state, data| ImeEvent::Composition(CompositionEvent { state, data });
        let event = match event {
            Ime::Enabled => composition(CompositionState::Start, String::new()),
            Ime::Preedit(text, _) => composition(CompositionState::Update, text.clone()),
            Ime::Commit(text) => composition(CompositionState::End, text.clone()),
            Ime::Disabled => ImeEvent::Dismissed,
        };
        self.webview.notify_input_event(InputEvent::Ime(event));
    }

    fn evaluate_script(&self, script: &str, callback: Box<dyn FnOnce(ScriptResult)>) {
        self.webview.evaluate_javascript(script, move |result| {
            callback(
                result
                    .map(|value| js_value_to_string(&value))
                    .map_err(|error| format!("{error:?}")),
            )
        });
    }

    fn url(&self) -> Option<Url> {
        self.webview.url()
    }

    fn title(&self) -> Option<String> {
        self.webview.page_title()
    }

    fn load_state(&self) -> LoadState {
        load_state(self.webview.load_status())
    }

    fn can_go_back(&self) -> bool {
        self.webview.can_go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.webview.can_go_forward()
    }

    fn spin(&self) {
        self.servo.spin_event_loop();
    }

    fn take_events(&self) -> Vec<EngineEvent> {
        self.delegate.events.take()
    }

    fn gl(&self) -> Arc<glow::Context> {
        self.view_context.glow_gl_api()
    }

    fn make_current(&self) {
        if let Err(error) = self.view_context.make_current() {
            log::warn!("OpenGL の文脈を使えない: {error:?}");
        }
    }

    fn resize_window(&self, size: PhysicalSize<u32>) {
        self.window_context.resize(size);
    }

    fn resize_view(&self, size: PhysicalSize<u32>) {
        if self.webview.size().to_u32() != euclid::Size2D::new(size.width, size.height) {
            self.webview.resize(size);
        }
    }

    fn paint(&self) {
        self.webview.paint();
    }

    fn composite_fn(&self) -> Option<CompositeFn> {
        let render_to_parent = self.view_context.render_to_parent_callback()?;
        Some(Box::new(move |gl, rect| {
            render_to_parent(
                gl,
                euclid::Rect::new(
                    euclid::Point2D::new(rect.x, rect.y),
                    euclid::Size2D::new(rect.width, rect.height),
                ),
            )
        }))
    }

    fn begin_frame(&self) {
        self.window_context.prepare_for_rendering();
    }

    fn present(&self) {
        self.window_context.present();
    }
}

/// Servo からの通知を受け取り、[`EngineEvent`] にして溜めておく。
#[derive(Default)]
struct Delegate {
    events: RefCell<Vec<EngineEvent>>,
    /// 今表示している IME。入力欄から入力欄へ移ると、新しい表示の後で古い表示の
    /// 取り消しが届くことがあるので、どれが今のものかを覚えておく
    input_method: Cell<Option<EmbedderControlId>>,
}

impl Delegate {
    fn push(&self, event: EngineEvent) {
        self.events.borrow_mut().push(event);
    }
}

impl WebViewDelegate for Delegate {
    fn notify_url_changed(&self, _webview: WebView, url: Url) {
        self.push(EngineEvent::UrlChanged(url));
    }

    fn notify_page_title_changed(&self, _webview: WebView, title: Option<String>) {
        self.push(EngineEvent::TitleChanged(title));
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        // 入力欄にフォーカスがあるまま別のページに移ると、Servo は hide_embedder_control を
        // 呼ばない。前のページの入力欄の IME が残らないよう、ここで消す
        if status == LoadStatus::Started && self.input_method.take().is_some() {
            self.push(EngineEvent::InputMethodHidden);
        }
        self.push(EngineEvent::LoadStateChanged(load_state(status)));
    }

    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.push(EngineEvent::NewFrameReady);
    }

    fn show_embedder_control(&self, _webview: WebView, control: EmbedderControl) {
        // <select> やダイアログなどは Phase 6 で扱う。いまは IME だけ
        if let EmbedderControl::InputMethod(input_method) = control {
            self.input_method.set(Some(input_method.id()));
            let rect = input_method.position();
            self.push(EngineEvent::InputMethodShown {
                area: PixelRect {
                    x: rect.min.x,
                    y: rect.min.y,
                    width: rect.width(),
                    height: rect.height(),
                },
            });
        }
    }

    fn hide_embedder_control(&self, _webview: WebView, control_id: EmbedderControlId) {
        if self.input_method.get() == Some(control_id) {
            self.input_method.set(None);
            self.push(EngineEvent::InputMethodHidden);
        }
    }
}

struct Waker(Arc<dyn Fn() + Send + Sync>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Waker(self.0.clone()))
    }

    fn wake(&self) {
        (self.0)();
    }
}

fn load_state(status: LoadStatus) -> LoadState {
    match status {
        LoadStatus::Started => LoadState::Started,
        LoadStatus::HeadParsed => LoadState::HeadParsed,
        LoadStatus::Complete => LoadState::Complete,
    }
}

fn js_value_to_string(value: &JSValue) -> String {
    match value {
        JSValue::Undefined => "undefined".into(),
        JSValue::Null => "null".into(),
        JSValue::Boolean(value) => value.to_string(),
        JSValue::Number(value) => value.to_string(),
        JSValue::String(value) => value.clone(),
        JSValue::Array(values) => {
            let values: Vec<_> = values.iter().map(js_value_to_string).collect();
            format!("[{}]", values.join(", "))
        }
        other => format!("{other:?}"),
    }
}
