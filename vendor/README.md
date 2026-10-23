# vendor：手を入れた Servo の crate

ここにある crate は、crates.io の版をそのまま写し、少しだけ直したものです。ルートの `Cargo.toml` の `[patch.crates-io]` で、crates.io の版の代わりに使っています。

| ディレクトリ | 元 | 直した内容 | 差分 |
| --- | --- | --- | --- |
| `servo-script/` | crates.io の `servo-script` 0.1.4（Servo の `components/script`） | ページの JS を動かさない | `servo-script.patch` |

直した箇所には、検索しやすいように `STROKE PATCH` と書いてあります。

## servo-script：ページの JS を動かさない

Servo 0.1.4 には、JS を止める設定（`Preferences` や `Opts`）がありません。そこで、HTML の仕様にある「サンドボックスの中の文書ではスクリプトを動かさない」仕組みを使います。

1. `dom/servoparser/mod.rs`：読み込んだ文書すべてに、サンドボックスの印 `SANDBOXED_SCRIPTS_BROWSING_CONTEXT_FLAG` を付ける。HTTP の応答に `Content-Security-Policy: sandbox`（`allow-scripts` なし）が付いていたのと同じ状態になる。Servo はこの印があると `<script>`、`onclick` などのイベントハンドラーの属性、iframe の中のスクリプトを動かさない（`Document::scripting_enabled`、`GlobalScope::can_run_script`）。iframe の中の文書は、親の文書の印を引き継ぐ
2. `dom/globalscope.rs`：`evaluate_js_on_global` の `assert!(self.can_run_script())` を、失敗を返す形に変える。元のままだと、スクリプトを動かせない文書で `javascript:` のリンクを開いたり、外枠が `WebView::evaluate_javascript` を呼んだりすると、assert でスクリプトのスレッドが落ちる。仕様でも、スクリプトを動かせないときは「何もしない」が正しい

サンドボックスの印のうち、付けるのはスクリプトの印だけです。オリジンの印（`SANDBOXED_ORIGIN`）は付けないので、Cookie や localStorage はふつうに使えます（JS がないので localStorage は使われませんが）。

### 確かめたこと（2026-10-23）

Xvfb の上で、次のページを開いて確かめました。どれも動かず、タイトルは書き換わらず、Stroke も落ちませんでした。

- インラインの `<script>`、外部の `<script src>`
- `<img onerror>` のイベントハンドラーの属性
- `<iframe srcdoc>` の中の `<script>`
- キーで選んで開いた `javascript:` のリンク

### 分かっている違い

- `<noscript>` の中身が出ない。HTML のパーサーは、作られたときに「スクリプトが有効か」を読むが、サンドボックスの印はその後に付くため。表示が困るページが出てきたら直す
- 外部のスクリプト（`<script src>`）は、実行はされないが、ダウンロードはされる（先読みのため）
- 外枠からの JS（`Engine::evaluate_script`）も動かない。Phase 3 のヒントは注入した JS で要素を集める予定なので、そのときに決め直す（`docs/decisions.md`）

## 版を上げるとき

Servo の LTS を上げるときは、新しい版の crate を取ってきて、差分を当て直します。

```sh
v=0.1.5   # 新しい版
rm -rf vendor/servo-script
mkdir -p vendor/servo-script
curl -sSfL "https://static.crates.io/crates/servo-script/servo-script-$v.crate" \
  | tar -xz -C vendor/servo-script --strip-components=1
patch -d vendor/servo-script -p1 < vendor/servo-script.patch
```

差分が当たらなければ、`STROKE PATCH` の2か所を手で直し、差分を作り直します（`diff -ru 元の crate vendor/servo-script > vendor/servo-script.patch`、パスは `a/` と `b/` に揃える）。

ビルドすると、servo-script の警告が4つ出ます。どれも Servo の元のコードのもので、crates.io から使っていたときは隠れていただけです。
