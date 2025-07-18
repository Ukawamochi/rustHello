# LED Control for Raspberry Pi Pico

このプロジェクトは、Raspberry Pi Picoの組み込みGPIOピン（GPIO25）を使用してLEDを制御するRustプログラムです。

## 概要

- **プラットフォーム**: Raspberry Pi Pico (RP2040)
- **言語**: Rust
- **機能**: GPIO25ピンのLEDを常時点灯

## 必要なツール

- Rust toolchain
- `thumbv6m-none-eabi` ターゲット
- `elf2uf2-rs` (UF2形式への変換用)

## セットアップ

1. Rustツールチェーンのインストール:
```sh
rustup target add thumbv6m-none-eabi
```

2. `elf2uf2-rs`のインストール:
```sh
cargo install elf2uf2-rs
```

## ビルドと実行

1. プロジェクトをビルド:
```sh
cargo build --release
```

2. UF2ファイルに変換:
```sh
elf2uf2-rs target/thumbv6m-none-eabi/release/led
```

3. Raspberry Pi PicoのBOOTSELボタンを押しながらUSBケーブルで接続し、生成された`.uf2`ファイルをドラッグ＆ドロップ

## 回路

- GPIO25ピンにLEDのアノード（+）を接続
- LEDのカソード（-）を適切な抵抗を通してGNDに接続

## 使用ライブラリ

- `rp-pico`: Raspberry Pi Pico HAL
- `embedded-hal`: 組み込みシステム用抽象化レイヤー
- `cortex-m-rt`: Cortex-Mランタイム
- `panic-halt`: パニック時の処理
