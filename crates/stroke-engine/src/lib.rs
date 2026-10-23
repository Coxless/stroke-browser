//! 外枠とエンジンの境界。
//!
//! 外枠（`crates/stroke`）は Servo の型を直接触らず、この crate の [`Engine`] trait だけを使う。
//! エンジンとのやり取りは「外枠からの操作」（trait のメソッド）と
//! 「エンジンからの通知」（[`EngineEvent`]）の2つに分けている。

mod keys;
mod servo_engine;

use std::sync::Arc;

pub use servo_engine::ServoEngine;
pub use url::Url;
use winit::dpi::PhysicalSize;
use winit::event::{Ime, KeyEvent};
use winit::keyboard::ModifiersState;

/// 読み込みの状態。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadState {
    /// 読み込みを始めたが、まだ `<head>` を読み終えていない
    Started,
    /// `<head>` を読み終えた。本文はまだ読み込み中
    HeadParsed,
    /// 文書と、そこから読む資源をすべて読み終えた
    Complete,
}

/// 画面上の長方形。単位は物理ピクセル、原点はウィンドウの左上。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// エンジンからの通知。[`Engine::take_events`] でまとめて受け取る。
#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    UrlChanged(Url),
    TitleChanged(Option<String>),
    LoadStateChanged(LoadState),
    /// 新しい画面が描けたので、ウィンドウを描き直してほしい
    NewFrameReady,
    /// ページの入力欄にフォーカスが入り、IME を使えるようにしてほしい。
    /// `area` は入力欄の位置で、ページの表示領域の左上を原点とする
    InputMethodShown {
        area: PixelRect,
    },
    /// 入力欄からフォーカスが外れた
    InputMethodHidden,
}

/// JS を実行した結果。文字列にして返す（型を細かく扱うのは必要になってから）。
pub type ScriptResult = Result<String, String>;

/// 画面に描いたページをウィンドウに写す関数。引数は写す先の長方形（OpenGL の座標系で、
/// 原点はウィンドウの左下）。
pub type CompositeFn = Box<dyn Fn(&glow::Context, PixelRect) + Send + Sync>;

/// 外枠から見たエンジン。いまは WebView 1枚だけを持つ。
pub trait Engine {
    // --- 外枠からの操作 ---

    fn load(&self, url: Url);
    fn go_back(&self);
    fn go_forward(&self);
    fn reload(&self);
    /// キーをページに渡す
    fn send_key(&self, event: &KeyEvent, modifiers: ModifiersState);
    /// IME の変換中の文字や確定した文字をページに渡す
    fn send_ime(&self, event: &Ime);
    /// ページで JS を実行する。結果は後から `callback` に渡す。
    /// いまはページの JS を止めているので、必ず失敗する（`vendor/README.md`）
    fn evaluate_script(&self, script: &str, callback: Box<dyn FnOnce(ScriptResult)>);

    // --- 今の状態 ---

    fn url(&self) -> Option<Url>;
    fn title(&self) -> Option<String>;
    fn load_state(&self) -> LoadState;
    fn can_go_back(&self) -> bool;
    fn can_go_forward(&self) -> bool;

    // --- エンジンからの通知 ---

    /// エンジンの仕事を進める。起こされたときと、ウィンドウのイベントのたびに呼ぶ
    fn spin(&self);
    /// 前回から溜まった通知を取り出す
    fn take_events(&self) -> Vec<EngineEvent>;

    // --- 表示 ---

    /// 外枠の UI も同じ OpenGL の文脈で描くので、それを渡す
    fn gl(&self) -> Arc<glow::Context>;
    fn make_current(&self);
    /// ウィンドウの大きさが変わった
    fn resize_window(&self, size: PhysicalSize<u32>);
    /// ページを表示する領域の大きさが変わった
    fn resize_view(&self, size: PhysicalSize<u32>);
    /// ページを画面外のバッファに描く
    fn paint(&self);
    /// 画面外のバッファをウィンドウに写す関数。まだ何も描いていなければ `None`
    fn composite_fn(&self) -> Option<CompositeFn>;
    /// ウィンドウへの描画を始める
    fn begin_frame(&self);
    /// 描いたものをウィンドウに出す
    fn present(&self);
}
