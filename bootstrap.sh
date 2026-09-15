#!/usr/bin/env bash
set -e

echo "==> Initializing RChain Sentinel..."

mkdir -p \
  backend/src \
  frontend/src \
  rholang \
  tests \
  docs \
  .github/workflows

cat > backend/Cargo.toml <<'EOF'
[package]
name = "rchain-sentinel-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
tower-http = { version = "0.6", features = ["cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
EOF

cat > backend/src/main.rs <<'EOF'
use axum::{
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "rchain-sentinel",
        version: "0.1.0",
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive());

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    println!("RChain Sentinel backend listening on {}", address);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
EOF

cat > frontend/package.json <<'EOF'
{
  "name": "rchain-sentinel-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build"
  },
  "dependencies": {
    "vite": "^7.0.0",
    "typescript": "^5.0.0"
  },
  "devDependencies": {}
}
EOF

cat > frontend/index.html <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>RChain Sentinel</title>
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
EOF

cat > frontend/src/main.ts <<'EOF'
const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Application root not found");
}

app.innerHTML = `
  <main>
    <h1>RChain Sentinel</h1>
    <p>Transaction, Contract & Network Verification Console</p>

    <section>
      <h2>System Status</h2>
      <div id="status">Checking backend...</div>
    </section>
  </main>
`;

async function checkBackend() {
  const status = document.querySelector<HTMLDivElement>("#status");

  if (!status) return;

  try {
    const response = await fetch("http://localhost:8080/health");
    const data = await response.json();

    status.textContent =
      `${data.service} — ${data.status} — v${data.version}`;
  } catch {
    status.textContent = "Backend unavailable";
  }
}

checkBackend();
EOF

cat > .gitignore <<'EOF'
target/
node_modules/
dist/
.env
.env.*
!.env.example
*.log
EOF

cat > README.md <<'EOF'
# RChain Sentinel

RChain transaction, contract, and network verification console.

## Vision

RChain Sentinel is an observability and verification layer for the RChain ecosystem.

Core pipeline:

Observe → Simulate → Verify → Execute

## Architecture

- Rust backend
- TypeScript frontend
- RNode integration
- Rholang analysis
- Transaction inspection
- Network observability
- Security analysis
- Future formal verification integration

## Status

Early development — v0.1.0
EOF

cat > .github/workflows/ci.yml <<'EOF'
name: CI

on:
  push:
    branches: ["main"]
  pull_request:

jobs:
  backend:
    runs-on: ubuntu-latest

    defaults:
      run:
        working-directory: backend

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Check
        run: cargo check

      - name: Test
        run: cargo test
EOF

echo "==> RChain Sentinel structure created."
echo "==> Backend: Rust + Axum"
echo "==> Frontend: TypeScript + Vite"
echo "==> CI: GitHub Actions"
echo "==> Done."
