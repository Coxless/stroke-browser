# 日本語入力（IME）

Phase 1 の作業2。Ubuntu で、ページの入力欄と URL 入力欄に IME（fcitx5 または IBus）で日本語を入力できるかを確かめ、結果をここに残します。

## 今の状態（2026-10-23）

**まだ画面で確かめていない。** Phase 1 のコードはクラウドの環境（画面なし）で書いたため、ビルドとテストまでは通したが、窓を出して IME で打つ確認はできていない。下の「確かめ方」を Ubuntu の Workshop で行い、「結果」を埋める。

ただし、Servo 0.1.4 のコードを読んだ限り、Servo 側には IME を受ける仕組みがある。外枠がつなぎさえすれば動くはずなので、Stroke の外枠には最初からつないである（下の「仕組み」）。

## 仕組み

Servo 0.1.4（`servo` crate と servoshell の `desktop/headed_window.rs`）を読んで分かったこと。

1. ページの入力欄にフォーカスが入ると、Servo が `WebViewDelegate::show_embedder_control` を `EmbedderControl::InputMethod` で呼ぶ。入力欄の位置（`position()`、WebView の左上が原点、物理ピクセル）が付いてくる
2. 外枠はそれを受けて `winit::Window::set_ime_allowed(true)` と `set_ime_cursor_area(…)` を呼ぶ。これで OS の IME が有効になり、変換候補の窓が入力欄のそばに出る
3. IME の入力は winit の `WindowEvent::Ime` で届く。外枠はそれを Servo の `InputEvent::Ime` に直して渡す
   - `Ime::Enabled` → `CompositionState::Start`
   - `Ime::Preedit(文字)` → `CompositionState::Update`（変換中の文字）
   - `Ime::Commit(文字)` → `CompositionState::End`（確定）
   - `Ime::Disabled` → `ImeEvent::Dismissed`
4. フォーカスが外れると、Servo が `hide_embedder_control` を呼ぶ。外枠は `set_ime_allowed(false)` に戻す

Stroke では、1 と 4 を `crates/stroke-engine/src/servo_engine.rs` の `Delegate` が `EngineEvent::InputMethodShown` / `InputMethodHidden` にして外枠に知らせ、2 と 3 を `crates/stroke/src/main.rs` が行う。URL 入力欄（egui）が開いている間は、IME のイベントは egui に渡す。egui-winit が自分で `set_ime_allowed` を呼ぶので、入力欄を閉じたらページの入力欄の分を戻している（`Running::apply_page_ime`）。

`show_embedder_control` は、Phase 2 の「入力欄にフォーカスが入った・出たことを知る」にもそのまま使えるはず。

## 確かめ方

Ubuntu の Workshop の中で行う。

```sh
workshop shell
cd /project
cargo run -p stroke -- https://www.google.com/   # 入力欄のあるページなら何でもよい
```

確かめること：

| | 見ること | 結果 |
| --- | --- | --- |
| 1 | ページの入力欄に入ると、IME を有効にできる（`Ctrl+Space`、半角/全角など） | 未 |
| 2 | 変換中の文字が入力欄に出る（下線つきか、ただの文字か） | 未 |
| 3 | `Enter` で確定した文字が入力欄に入る。確定の `Enter` でフォームが送られてしまわない | 未 |
| 4 | 変換候補の窓が入力欄のそばに出る（画面の隅に出ないか） | 未 |
| 5 | URL 入力欄（`Ctrl+L`）でも 1〜4 が同じようにできる | 未 |
| 6 | 入力欄から出ると、IME が無効になる（ノーマルモードのキーを IME に取られない） | 未 |
| 7 | 同じことを servoshell でも試し、Stroke との違いがあれば書く（お手本との比べ） | 未 |

試す組み合わせ：fcitx5 と IBus、Wayland と X11（ログイン画面のセッションの選択で切り替える）。

### コンテナの中から IME に届くか

Workshop のコンテナの中のプログラムがホストの IME とやり取りできるかは、まだ分からない。

- **Wayland**：IME は Wayland の text-input の仕組みを通る。desktop インターフェースで Wayland につながっていれば、コンテナの中からでも使えるはず
- **X11**：XIM か、IME のデーモンとの D-Bus 越しのやり取りになる。`XMODIFIERS`（`@im=fcitx` など）が中に渡っているか、`echo $XMODIFIERS` で確かめる

使えなかった場合は、まずホストに直接ビルドしたもので試し、IME 自体の問題かコンテナの問題かを切り分ける。

## 結果

（確かめたら、ここに日付と結果を書く）

## 使えなかった場合

Servo 側の仕組み（上の 1〜4）が足りない場合は、外枠側で winit の IME イベントを受けて、変換中の文字を外枠（egui）がページの上に重ねて描き、確定した文字だけをページに渡す方法を考える。その作業は Phase 2 に足す。
