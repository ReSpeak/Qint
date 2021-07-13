$Env:FRONTEND_PATH = "./ui/"
$workhome = Get-Location
# Check if SDL exists
if (-Not (Get-Item "./proxy-codegen/SDL2.dll" -ErrorAction SilentlyContinue)) {
    ./install_sdl.ps1
}
# Build proxy
Set-Location(Join-Path $workhome "webapp")
cargo build --release
# Build frontend
Set-Location(Join-Path $workhome "frontend")
yarn
yarn build
# package
Set-Location($workhome)
New-Item "./target/publish/ui" -ItemType "directory" -Force | Out-Null
# Copy proxy
Copy-Item -Path "./target/release/webapp.exe" -Destination "./target/publish/qint.exe" -Force
# Foreach ($file in "webapp.exe", "WebView2Loader.dll") {
#     Copy-Item -Path "./target/release/$file" -Destination "./target/publish/" -Force
# }
Copy-Item -Path "./proxy-codegen/SDL2.dll" -Destination "./target/publish/" -Force
# Copy frontend
Copy-Item -Path "./frontend/dist/*" -Destination (Join-Path "./target/publish/" $Env:FRONTEND_PATH -Resolve) -Recurse -Force
