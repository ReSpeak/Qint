$Env:FRONTEND_PATH = "./ui/"
$workhome = Get-Location
# Check if SDL exists
if (-Not (Get-Item "./proxy/SDL2.dll")) {
    ./install_sdl.ps1
}
# Build proxy
Set-Location(Join-Path $workhome "proxy")
cargo build --release
# Build frontend
Set-Location(Join-Path $workhome "frontend")
yarn
yarn build
# package
Set-Location($workhome)
New-Item "./target/publish/ui" -ItemType "directory" -Force | Out-Null
# Copy proxy
Foreach ($file in "qint-proxy.exe", "SDL2.dll", "WebView2Loader.dll") {
    Copy-Item -Path "./target/release/$file" -Destination "./target/publish/" -Force
}
# Copy frontend
Copy-Item -Path "./frontend/build/*" -Destination (Join-Path "./target/publish/" $Env:FRONTEND_PATH -Resolve) -Recurse -Force
