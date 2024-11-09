$version = "2.30.9"
$zipDownloadPath = "./target/sdl.zip"
# Reuse rust build dir
New-Item -Path "./target" -ItemType "directory" -Force | Out-Null
# Download zip
if (Get-Item $zipDownloadPath -ErrorAction SilentlyContinue) {
    Remove-Item $zipDownloadPath -Force
}
Invoke-WebRequest -Uri "https://www.libsdl.org/release/SDL2-devel-$version-VC.zip" -OutFile $zipDownloadPath
# Extract and move all
Expand-Archive -Path $zipDownloadPath -DestinationPath "./target/sdl/" -Force
New-Item "./proxy-codegen/msvc/lib/64" -ItemType "directory" -Force | Out-Null
New-Item "./proxy-codegen/msvc/dll/64" -ItemType "directory" -Force | Out-Null
Copy-Item -Path "./target/sdl/SDL2-$version/lib/x64/*" -Filter "*.lib" -Destination "./proxy-codegen/msvc/lib/64/"
Copy-Item -Path "./target/sdl/SDL2-$version/lib/x64/*" -Filter "*.dll" -Destination "./proxy-codegen/msvc/dll/64/"
Copy-Item -Path "./target/sdl/SDL2-$version/lib/x64/*" -Filter "*.dll" -Destination "./proxy-codegen/"
# Cleanup
Remove-Item $zipDownloadPath -Force
Remove-Item "./target/sdl" -Recurse -Force
