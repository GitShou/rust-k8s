# 1段目: ビルド用イメージ
FROM rust:1.82-bullseye AS builder

WORKDIR /app

# 依存関係のキャッシュを効かせるために先にCargo.*だけコピー
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 本物のソースをコピーしてビルド
COPY src ./src
RUN cargo build --release

# 2段目: 実行用の軽量イメージ
FROM debian:bullseye-slim

# 必要に応じて最低限のライブラリだけ入れる（ログ出すだけならほぼ不要）
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Rustバイナリをコピー（バイナリ名はCargo.tomlのpackage.name）
COPY --from=builder /app/target/release/battle_game /app/battle_game

# 非rootユーザで動かしたい場合
RUN useradd -m appuser
USER appuser

CMD ["/app/battle_game"]
