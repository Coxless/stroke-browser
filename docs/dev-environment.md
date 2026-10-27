# 開発環境

Ubuntu では Canonical Workshop、Windows では WSL container を使います。どちらも、開発に要るものをコンテナの中に閉じ込め、手元の OS を汚さないためです。2つの環境に入るものは揃えてあり、同じ確認スクリプト（`scripts/check-env.sh`）で確かめます。

| | Ubuntu | Windows |
| --- | --- | --- |
| 使うもの | Canonical Workshop（LXD のコンテナ） | WSL container（`wslc`） |
| 定義 | `.workshop/dev.yaml`、`.workshop/stroke-dev/` | `container/Containerfile` |
| 操作 | `workshop` コマンド | `scripts/wslc-dev.ps1` |
| ソースの場所（中から見て） | `/project` | `/project` |
| 画面の表示 | desktop インターフェースで、ホストの表示につなぐ | WSLg を渡す（要確認） |
| 位置づけ | 主な開発環境 | 後回し。動作確認は Ubuntu で行う |

入るもの：Python 3、tkinter、Rust（rustup）、日本語フォント（Noto CJK）、git、ビルドの基本道具（Phase 0）、Servo のビルドに要るもの（Phase 1、下の「Servo のビルドに要るもの」）。

## Ubuntu：Canonical Workshop

### 必要なもの

- Ubuntu（デスクトップ環境あり）
- LXD 6.8 以上（snap の `6/stable`）

### 入れる

```sh
sudo snap install --channel=6/stable lxd    # 入っていなければ
sudo snap refresh --channel=6/stable lxd    # 古い版が入っていれば
sudo snap install --classic workshop
```

LXD のデーモンが有効で、動いていることを確かめておきます。

### 起動と確認

リポジトリの直下で実行します。

```sh
workshop launch              # SDK を入れて、環境を作る（初回だけ）
workshop run -- check        # 開発環境を確認する
workshop run -- check --gui  # 窓も出して確認する
workshop shell               # 中でシェルを開く
```

リポジトリは中の `/project` にそのまま見えます。中で書き換えたファイルは外にも反映されます。

### 日々の操作

| したいこと | コマンド |
| --- | --- |
| 止める・動かす | `workshop stop` / `workshop start` |
| 定義の変更を反映する、SDK を更新する | `workshop refresh` |
| 最後に launch・refresh した状態に戻す | `workshop restore` |
| 状態を見る | `workshop info` |
| つながりを見る | `workshop connections` |

`actions` の変更は `workshop run` のたびに読み直されるので、refresh は要りません。SDK やパッケージを変えたときは refresh します。

### 構成

- `.workshop/dev.yaml`：環境の定義。Ubuntu 24.04 に、次の2つの SDK を入れる
  - `rust`：SDK Store の Rust SDK。rustup と標準の道具が入り、ツールチェインと Cargo のキャッシュはホストに保存されて、refresh しても消えない
  - `project-stroke-dev`：このリポジトリ専用の SDK（`.workshop/stroke-dev/`）
- `.workshop/stroke-dev/sdk.yaml`：`desktop`（ホストの表示）と `gpu`（ホストの GPU）につなぐ。ブラウザを画面に出すために要る
- `.workshop/stroke-dev/hooks/setup-base`：launch と refresh のときに root で動き、apt でパッケージを入れる
- `.workshop.lock`：Workshop がプロジェクトごとに作る。リポジトリに入れない（`.gitignore` 済み）

## Servo のビルドに要るもの

Phase 1 で足しました。一覧は Servo の [`python/servo/platform/linux_packages/apt/apt_common.txt`](https://github.com/servo/servo/blob/v0.1.3/python/servo/platform/linux_packages/apt/apt_common.txt)（v0.1.3。`./mach bootstrap` が入れるもの）をもとにし、次のものを除いています。

- GStreamer の一式（`gstreamer1.0-*`、`libgstreamer-*-dev`、`libges-1.0-dev`、`libgstrtspserver-1.0-dev`）：`servo` crate の `media-gstreamer` 機能を使わないので要らない。動画と音声を扱うときに足す
- `ccache`：なくてもビルドできる
- `curl`、`git`、`build-essential`、`ca-certificates`：Phase 0 の分で入っている

足したものは `.workshop/stroke-dev/hooks/setup-base` と `container/Containerfile` の両方に同じ一覧で書いています。片方を変えたら、もう片方も変えます。`scripts/check-env.sh` は clang、cmake、主なライブラリ（pkg-config で見えるか）を確かめます。

SpiderMonkey（`mozjs_sys`）は、ビルド済みのものを [servo/mozjs の Releases](https://github.com/servo/mozjs/releases) から落としてくるので、手元ではビルドしません。ネットにつながらない環境では `MOZJS_FROM_SOURCE=1` でソースからビルドします（時間がかかります）。

## Stroke をビルドして動かす

リポジトリの直下で（Workshop なら `workshop shell` の中の `/project` で）実行します。

```sh
cargo build                      # 初回は Servo の全部をビルドするので時間がかかる
cargo test                       # 外枠とエンジンの境界の単体テスト
cargo run -p stroke -- servo.org # URL を渡して起動する。省くと https://servo.org/ を開く
```

目安（2026-10-23、4コア・メモリ 15GB のクラウドの環境で測った）：初回の `cargo build` は約15分、`target/` は約4.5GB。2回目からは自分の crate だけをビルドするので数秒。メモリは 15GB で足りた。

### 安全に動かす

Stroke は作りかけで、安全のための修正を追いかける体制もありません。次の3つを守って動かします。

- **Workshop の中で動かす**。ホストで直接動かさない。中からはホームディレクトリなどに届かない
- **X11 を使わない**。X11 につながったプログラムは、ほかの窓のキー入力を盗み見できる。`DISPLAY` を外し、Wayland だけで表示する
- **ログインしない**。実際に使っているアカウントにログインしたり、パスワードを入れたりしない

ホストから、次のように起動します。`DISPLAY` を外し、Wayland につながっていなければ起動しません。

```sh
workshop run -- run-browser servo.org
```

終えるときは窓を閉じます。外から `workshop run` を止めても、中でブラウザが動き続けることがあります。そのときは `workshop exec -- pkill -x stroke` で止めます。

`desktop` インターフェースがつながっていないと、中から画面が見えません。`workshop connections` で `desktop` の行に SLOT が出ていなければ、`workshop connect dev/stroke-dev:desktop` でつなぎます。

`/project` は中から書き込めるので、乗っ取られたブラウザが `.git/hooks/` などに書き込むと、ホストで `git` を使ったときに動いてしまいます。気になるページを開いた後は、ホストで `git status` と `ls .git/hooks | grep -v sample` を確かめます。

起動すると、ウィンドウの下に読み込みの状態、URL、仮のキーの一覧が出ます。キーの一覧は `docs/decisions.md` の「Phase 1 の仮のキー」にあります。`RUST_LOG=stroke=debug` を付けると、押したキーの行き先（外枠の命令か、ページか）とエンジンからの通知がログに出ます。

### servoshell（お手本）を動かす

`crates/stroke` は servoshell と同じ作りなので、動きを比べたいときは servoshell を動かします。servoshell は crates.io にないので、Servo のリポジトリを取ってきて `./mach` でビルドします。LTS の 0.1.4 には git のタグがないので、一番近い `v0.1.3` を使います。

```sh
git clone --depth 1 --branch v0.1.3 https://github.com/servo/servo.git ~/servo
cd ~/servo
curl -LsSf https://astral.sh/uv/install.sh | sh   # mach が使う。入っていなければ
./mach build --release
./mach run --release https://servo.org/
```

`./mach bootstrap` は apt で GStreamer なども入れます。上の一覧で足りない分を入れたいときだけ使います。Servo のリポジトリはこのリポジトリの外（`~/servo`）に置きます。

## Windows：WSL container

### 必要なもの

- WSL 2.9.3 以上。`wsl --version` で確かめ、古ければ `wsl --update`
- `wslc version` が動くこと

### 起動と確認

PowerShell で、リポジトリの直下で実行します。

```powershell
.\scripts\wslc-dev.ps1 build          # イメージを作る（初回と Containerfile を変えたとき）
.\scripts\wslc-dev.ps1 check          # 開発環境を確認する
.\scripts\wslc-dev.ps1 check -Gui     # 窓も出して確認する
.\scripts\wslc-dev.ps1 shell          # 中でシェルを開く
```

リポジトリは中の `/project` に見えます。Rust のビルドの成果物は、Windows 側のファイルの遅さを避けるため、コンテナの中（`/home/dev/target`）に置きます。コンテナは使うたびに作り直すので、成果物は残りません。遅さが気になってきたら、残す方法を考えます。

CPU の数とメモリの上限は、WSL container の設定（`session.cpuCount`、`session.memorySize`）で変えられます。既定ではメモリは Windows の半分です。Phase 1 で Servo をビルドするときに、足りるかを確かめます。

### まだ確かめていないこと

公式の文書に書かれていないので、最初の確認で試します。結果はこの節に書き足します。

- [ ] `-v` でのフォルダのマウントが、この書き方（`C:/...:/project`）で通るか
- [ ] **GUI**：`-Gui` は、WSLg の公式の例（Docker で `/mnt/wslg` と `/tmp/.X11-unix` を渡す）を `wslc` に当てはめたもの。`wslc` のコンテナから WSLg が見えるかは分からない
- [ ] 公式のチュートリアルは、ファイルを WSL 側に置くことを勧めている。Windows 側に置いて遅すぎないか
- [ ] Servo の初回ビルドは約15分かかる（Ubuntu の目安）。成果物（`/home/dev/target`）はコンテナと一緒に消えるので、毎回やり直しになる。名前付きのボリュームなどで残せるか

GUI が使えなかった場合、Phase 0 のトイブラウザは Windows に直接入れた Python で動かします（tkinter は Windows の Python に入っています）。コンテナはビルドとテストに使います。

## 参考にした公式の文書

- [Canonical Workshop documentation](https://ubuntu.com/workshop/docs/)
  - [Tutorial Part 1: Get started](https://ubuntu.com/workshop/docs/tutorial/part-1-get-started/)
  - [Tutorial Part 2: Work with interfaces](https://ubuntu.com/workshop/docs/tutorial/part-2-work-with-interfaces/)
  - [Workshop definition](https://ubuntu.com/workshop/docs/reference/definition-files/workshop-definition/)
  - [SDK definition](https://ubuntu.com/workshop/docs/reference/definition-files/sdk-definition/)
  - [SDKs（hooks）](https://ubuntu.com/workshop/docs/reference/sdks/)
- [canonical/workshop](https://github.com/canonical/workshop)、[canonical/rust-sdk](https://github.com/canonical/rust-sdk)
- [WSL container](https://learn.microsoft.com/en-us/windows/wsl/wsl-container)
- [Get started with containers on WSL](https://github.com/MicrosoftDocs/wsl/blob/main/WSL/tutorials/wsl-containers.md)
- [WSL container settings](https://github.com/MicrosoftDocs/wsl/blob/main/WSL/wsl-container-settings.md)
- [WSLg：Containerizing GUI applications](https://github.com/microsoft/wslg/blob/main/samples/container/Containers.md)
