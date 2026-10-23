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

入るもの（Phase 0 の時点）：Python 3、tkinter、Rust（rustup）、日本語フォント（Noto CJK）、git、ビルドの基本道具。Servo のビルドに要るものは、Phase 1 で Servo の文書を確かめてから足します。

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
