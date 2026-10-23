# Windows 用の開発環境（WSL container）を操作する。手順は docs/dev-environment.md
#
#   .\scripts\wslc-dev.ps1 build          # イメージを作る
#   .\scripts\wslc-dev.ps1 shell          # コンテナの中でシェルを開く
#   .\scripts\wslc-dev.ps1 check          # 開発環境を確認する
#   .\scripts\wslc-dev.ps1 check -Gui     # 窓も出して確認する
#
# -Gui を付けると、WSLg の表示をコンテナに渡す。wslc で WSLg が使えるかは
# 公式の文書に書かれていないので、最初の check -Gui で確かめる。

param(
    [Parameter(Position = 0)]
    [ValidateSet('build', 'shell', 'check')]
    [string]$Command = 'shell',
    [switch]$Gui
)

$ErrorActionPreference = 'Stop'
$Image = 'stroke-dev'
# マウント元は C:/... の形で渡す（バックスラッシュを受け付けない版があるため）
$Root = (Split-Path -Parent $PSScriptRoot) -replace '\\', '/'

function Invoke-DevContainer([string[]]$CommandArgs) {
    $runArgs = @('run', '--rm', '-it', '-v', "${Root}:/project")
    if ($Gui) {
        # WSLg の例（microsoft/wslg の samples/container）と同じ渡し方
        $runArgs += @(
            '-v', '/mnt/wslg:/mnt/wslg',
            '-v', '/tmp/.X11-unix:/tmp/.X11-unix',
            '-e', 'DISPLAY=:0',
            '-e', 'WAYLAND_DISPLAY=wayland-0',
            '-e', 'XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir',
            '-e', 'PULSE_SERVER=unix:/mnt/wslg/PulseServer'
        )
    }
    & wslc @runArgs $Image @CommandArgs
    exit $LASTEXITCODE
}

switch ($Command) {
    'build' {
        & wslc build -t $Image (Join-Path $Root 'container')
        exit $LASTEXITCODE
    }
    'shell' { Invoke-DevContainer @('bash') }
    'check' {
        $checkArgs = @('bash', '/project/scripts/check-env.sh')
        if ($Gui) { $checkArgs += '--gui' }
        Invoke-DevContainer $checkArgs
    }
}
