#!/usr/bin/env bash
#
# 薄启动器签名 + 公证（PLAN §8 / §9）。
# 前置：Xcode CLT、有效的 "Developer ID Application" 证书、
#       App Store Connect API 密钥（.p8 + key id + issuer id）。
#
# 用法：
#   APP_PATH=target/release/bundle/macos/DeepSeek Harness.app \
#   ./sign.sh
#
# 注意：entitlements.plist 已关闭 App Sandbox（开发者工具类 app 需关 Sandbox
# 才能 spawn 外部 dsh）。Hardened Runtime 由 --options runtime 启用。

set -euo pipefail

APP_PATH="${APP_PATH:?请设置 APP_PATH 指向 .app}"
BUNDLE_ID="com.deepseek.harness.desktop"

# —— 从环境变量读取，不要硬编码密钥 ——
APPLE_KEY_ID="${APPLE_KEY_ID:?设置 APPLE_KEY_ID（App Store Connect Key ID）}"
APPLE_ISSUER="${APPLE_ISSUER:?设置 APPLE_ISSUER（Issuer ID）}"
APPLE_KEY_PATH="${APPLE_KEY_PATH:?设置 APPLE_KEY_PATH（.p8 文件路径）}"
TEAM_ID="${TEAM_ID:?设置 TEAM_ID}"
CERT_NAME="${CERT_NAME:-Developer ID Application: DeepSeek Harness ($TEAM_ID)}"

ENTITLEMENTS="src-tauri/entitlements.plist"

echo "==> codesign (Hardened Runtime ON, Sandbox OFF)"
codesign --force --options runtime --timestamp \
  --entitlements "$ENTITLEMENTS" \
  --sign "$CERT_NAME" "$APP_PATH"

echo "==> verify"
codesign --verify --verbose --strict "$APP_PATH"
spctl -a -vv "$APP_PATH" || true

echo "==> notarize"
xcrun notarytool submit "$APP_PATH" \
  --key "$APPLE_KEY_PATH" \
  --key-id "$APPLE_KEY_ID" \
  --issuer "$APPLE_ISSUER" \
  --wait

echo "==> staple"
xcrun stapler staple "$APP_PATH"
echo "==> done: $APP_PATH"
