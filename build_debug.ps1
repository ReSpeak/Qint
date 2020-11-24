$Env:FRONTEND_PATH = "../../frontend/build/"
$workhome = Get-Location
# Check if SDL exists
if (-Not (Get-Item "./proxy/SDL2.dll" -ErrorAction SilentlyContinue)) {
    ./install_sdl.ps1
}
# Build proxy
cargo build
Copy-Item "./proxy/SDL2.dll" -Destination "./target/debug/"
# Build frontend
Set-Location(Join-Path $workhome "frontend")
yarn
yarn upgrade
yarn build
# Go back
Set-Location $workhome