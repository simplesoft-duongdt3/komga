#!/bin/bash
set -e

echo "Building komga-webui..."
cd ../komga-webui
npm ci
npm run build
cd -

echo "Copying webui to komga-rust..."
rm -rf webui-dist
cp -r ../komga-webui/dist webui-dist

echo "Building komga-rust..."
cargo build --release

echo "Build complete!"
echo "Run: ./target/release/komga-rust"