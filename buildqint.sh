#!/bin/bash

set -e
. ~/.bashrc
mkdir -p log out
exec 1> "log/$QINT_LOG_FILE"
exec 2>&1

export FRONTEND_PATH="./ui/"

echo "Build Started"

echo ">>> Cloning"
if [[ ! -d "Qint" ]]; then
        git clone git@git_splamy:ReSpeak/Qint.git
fi
if [[ ! -d "tsclientlib" ]]; then
        git clone https://github.com/ReSpeak/tsclientlib --recursive
fi

echo ">>> Pulling"
git -C Qint fetch
git -C tsclientlib fetch

echo ">>> Checking out '$QINT_SHA'"
git -C Qint checkout -f "$QINT_SHA"
git -C tsclientlib checkout origin/master
git -C tsclientlib submodule update --init --recursive

echo ">>> Testing"
cd ~/Qint/webapp
#cargo test --release

echo ">>> Building"
cd ~/Qint/webapp
RUSTFLAGS="-C link-args=-lssp -C link-args=-s" cargo build --release --target=x86_64-pc-windows-gnu

cd ~/Qint/frontend
yarn
yarn build

echo ">>> Packaging"
cd ~
rm -rf ~/Qint/target/publish
mkdir -p ~/Qint/target/publish/ui
cp ~/Qint/target/x86_64-pc-windows-gnu/release/webapp.exe ~/Qint/target/publish/
#cp ~/Qint/target/x86_64-pc-windows-gnu/release/WebView2Loader.dll ~/Qint/target/publish/
cp ~/Qint/proxy-codegen/SDL2.dll ~/Qint/target/publish/
cp /usr/lib/gcc/x86_64-w64-mingw32/10-win32/libssp-0.dll ~/Qint/target/publish/
cp -r ~/Qint/frontend/dist/* ~/Qint/target/publish/ui/
cd ~/Qint/target/publish/
rm ~/out/Qint.zip
mv webapp.exe qint.exe
zip ~/out/Qint.zip ui qint.exe libssp-0.dll SDL2.dll -r -9

echo ">>> Done!"
