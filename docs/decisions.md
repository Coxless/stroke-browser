# 決めたこと

決めた日付と理由を残します。新しいものを下に足します。決め直したときは、古いものを消さずに「→ 〇〇で決め直し」と書き添えます。

## 2026-10-23 外枠の UI は winit ＋ egui で作る（Phase 1）

**決めたこと**：外枠の言語は Rust。ウィンドウとイベントは winit、外枠の UI（URL 入力欄、状態表示、これから作るヒントのラベルなど）は egui で描く。egui は egui_glow で、Servo と同じ OpenGL の文脈に描く。

**比べたもの**

| | winit ＋ egui | 外枠の UI も Web 技術で描く（2枚目の WebView） |
| --- | --- | --- |
| キーの経路 | winit からの1本だけ。外枠が最初に受け取り、ページに渡すかを決める | 外枠の UI 用の WebView と、ページの WebView の2本。どちらが先に受け取るかの調整が要る |
| お手本 | servoshell が同じ作り。`ports/servoshell/desktop/gui.rs` を読みながら作れる | Servo には例がない |
| UI の作り込み | egui の部品で足りる範囲なら速い。凝った見た目は苦手 | HTML と CSS で自由に作れる |
| ページとの隔たり | 外枠の UI はページから見えず、ページの CSS の影響も受けない | 同じエンジンで描くので、エンジンの不具合が外枠にも及ぶ |

**理由**

- この計画の心臓部は「すべてのキーを外枠が最初に受け取る」こと（Phase 2）。キーを最初に受け取る場所を1つに保てる winit ＋ egui のほうが、この形に素直に合う
- servoshell と同じ作りなので、困ったときに読めるお手本がある
- Phase 3 のヒントのラベルは「ページの上に外枠が重ねて描く」予定で、egui ならページの層の上に重ねるだけで済む

**作り方の要点**：ページは Servo が画面外のバッファ（`OffscreenRenderingContext`）に描き、egui の一番下の層（`LayerId::background()`）にそれを写す。egui の UI はその上に描く。servoshell と同じ方法。

## 2026-10-23 Servo は crates.io の LTS 版（0.1.x）を使う（Phase 1）

**決めたこと**：`servo = "=0.1.4"` を crates.io から使う。git の main は追わない。

**理由**

- plan.md の「危険と対策」のとおり、API が版ごとに変わる危険を LTS で抑える。0.1.x は LTS の系列で、毎月の 0.2〜0.7 と並行して修正版が出ている（2026年10月の時点で最新は 0.7.0、LTS は 0.1.4）。[Servo Book の LTS の説明](https://book.servo.org/embedding/lts-release.html)によると、LTS は6か月ごとに新しく切られ、それぞれ約9か月、安全のための修正だけを受ける。0.1 は2026年4月に始まったので、2027年1月ごろまで
- crates.io から使えば、Servo のリポジトリを丸ごと取ってこなくても、`cargo build` だけでビルドできる。SpiderMonkey（`mozjs_sys`）はビルド済みのものを GitHub から落としてくるので、手元でビルドしなくて済む
- Phase 5 でエンジンを変えるときは、Servo を fork し、`[patch.crates-io]` で自分の版に差し替える

**上げる時期**：区切りのよい時（Phase の終わり）に、次の LTS が出ていれば上げる。0.1 の支えが切れる2027年1月より前に、次の LTS へ移る。上げるときは、Servo の型を閉じ込めている `crates/stroke-engine` だけを直せば済むはず。

## 2026-10-23 エンジンの境界に winit と glow の型は出してよい（Phase 1）

**決めたこと**：`Engine` trait（`crates/stroke-engine`）の引数と戻り値に Servo の型は出さない。ただし winit（キーと IME のイベント、大きさ）と glow（OpenGL）の型は出してよい。

**理由**：winit と glow は外枠も使っていて、エンジンを差し替えても変わらない。これらまで自前の型に包むと、変換の手間ばかり増えて、境界の見通しはよくならない。Servo の型（`WebView`、`InputEvent`、`LoadStatus` など）は外枠に漏らさず、`servo_engine.rs` と `keys.rs` の中に閉じ込める。

## 2026-10-23 Phase 1 の仮のキー（Phase 1）

**決めたこと**：Phase 2 でキー配置を決めるまでは、ページの文字入力とぶつからない修飾キー付きのキーと F キーだけを使う（`crates/stroke/src/shortcuts.rs`）。

| キー | 命令 |
| --- | --- |
| `Ctrl+L`、`Alt+D`、`F6` | URL 入力欄を開く |
| `Enter` / `Esc`（URL 入力欄で） | 開く / 閉じる |
| `Alt+←` / `Alt+→` | 戻る / 進む |
| `F5`、`Ctrl+R` | 再読み込み |
| `Ctrl+Q` | 終了 |

それ以外のキーはすべてページに渡す。スクロールは、ページに渡したキー（矢印、`Space`、`PageDown` など）を Servo が既定の動作として処理するのに任せる。

## 2026-10-23 URL でない入力は DuckDuckGo の HTML 版で検索する（Phase 1）

**決めたこと**：URL 入力欄に URL でないものを打ったら、`https://duckduckgo.com/html/?q=…` で検索する。設定で変えられるようにするのは Phase 6。

**理由**：JS を使わない HTML 版は、Servo でも崩れにくい。URL とみなす規則（スキームあり、`/` で始まるパス、ドメインらしいもの、`localhost` と IP アドレス）は `crates/stroke/src/location.rs` のテストにまとめてある。

## 2026-10-23 依存の crate だけ最適化してビルドする（Phase 1）

**決めたこと**：`Cargo.toml` の `[profile.dev.package."*"]` で、依存の crate を `opt-level = 2`、デバッグ情報なしでビルドする。自分の crate は通常のデバッグビルドのまま。

**理由**：最適化なしの Servo は遅すぎて、ページを読むのにも使えない。依存だけ最適化しておけば、自分のコードの変更は速くビルドでき、デバッガでも追える。デバッグ情報を外すのは、ディスクとリンクの時間を抑えるため。
