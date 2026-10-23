#!/bin/bash
# 開発環境の確認。Workshop では `workshop run -- check`、WSL container では
# `scripts/wslc-dev.ps1 check` から呼ぶ。--gui を付けると、窓を3秒だけ出す。
set -u

status=0
ok() { printf '  ok    %s\n' "$1"; }
ng() { printf '  NG    %s\n' "$1"; status=1; }

echo "Stroke browser 開発環境の確認"

if python3 --version >/dev/null 2>&1; then ok "$(python3 --version)"; else ng "python3 がない"; fi
if python3 -c 'import tkinter' 2>/dev/null; then ok "tkinter"; else ng "tkinter がない（python3-tk）"; fi
if command -v cargo >/dev/null; then ok "$(cargo --version)"; else ng "cargo がない"; fi
if command -v rustc >/dev/null; then ok "$(rustc --version)"; else ng "rustc がない"; fi
if command -v clang >/dev/null; then ok "$(clang --version | head -n 1)"; else ng "clang がない（Servo のビルドに要る）"; fi
if command -v cmake >/dev/null; then ok "$(cmake --version | head -n 1)"; else ng "cmake がない（Servo のビルドに要る）"; fi
servo_libs="freetype2 harfbuzz egl glesv2 x11 dbus-1 libudev"
if pkg-config --exists $servo_libs 2>/dev/null; then ok "Servo の依存ライブラリ"; else ng "Servo の依存ライブラリが足りない（pkg-config --exists $servo_libs）"; fi
if fc-list :lang=ja 2>/dev/null | grep -q .; then ok "日本語フォント"; else ng "日本語フォントがない（fonts-noto-cjk）"; fi

if [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ]; then
  ok "表示 DISPLAY=${DISPLAY:-} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-}"
else
  ng "表示につながっていない（DISPLAY も WAYLAND_DISPLAY もない）"
fi

if [ "${1:-}" = "--gui" ]; then
  python3 - <<'PY' && ok "tkinter の窓を出せた" || ng "tkinter の窓を出せない"
import tkinter as tk
root = tk.Tk()
root.title("Stroke browser")
tk.Label(root, text="Stroke browser 日本語の表示", font=("Noto Sans CJK JP", 20)).pack(padx=40, pady=40)
root.after(3000, root.destroy)
root.mainloop()
PY
fi

exit $status
