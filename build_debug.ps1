$Env:FRONTEND_PATH = "../../frontend/build/"
$workhome = Get-Location
# Build proxy
cargo build
# Build frontend
Set-Location(Join-Path $workhome "frontend")
yarn
yarn upgrade
yarn build
# Go back
Set-Location $workhome
