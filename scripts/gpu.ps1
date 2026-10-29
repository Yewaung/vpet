# Runs a command in the environment a GPU (Vulkan) build needs on Windows:
#   .\scripts\gpu.ps1 cargo build -p vpet --features gpu
#   .\scripts\gpu.ps1 npm run tauri dev -- --features gpu
#   .\scripts\gpu.ps1 cargo test -p vpet-llm --features vulkan
#
# Why each piece is needed (llama.cpp's Vulkan backend builds a helper tool,
# vulkan-shaders-gen, as a nested CMake project):
# - Visual Studio developer environment: the nested project must find cl.exe.
# - Ninja generator: with the Visual Studio generator, MSBuild runs the nested
#   project's configure and install steps at the same time and the build fails.
# - A short target folder (C:\vt): the nested paths pass Windows' 260-character
#   limit under a normal checkout, which MSBuild's file tracker can't handle.
# - VULKAN_SDK: from the Vulkan SDK installer (winget install KhronosGroup.VulkanSDK).

param([Parameter(Mandatory, ValueFromRemainingArguments)][string[]]$Command)
$ErrorActionPreference = "Stop"

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "Visual Studio C++ build tools not found." }
$vcvars = Join-Path $vs "VC\Auxiliary\Build\vcvars64.bat"
$ninja = Join-Path $vs "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"

$sdk = $env:VULKAN_SDK
if (-not $sdk) { $sdk = [Environment]::GetEnvironmentVariable("VULKAN_SDK", "Machine") }
if (-not $sdk) { $sdk = (Get-ChildItem C:\VulkanSDK -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1).FullName }
if (-not $sdk) { throw "Vulkan SDK not found. Install it: winget install KhronosGroup.VulkanSDK" }

$target = if ($env:VPET_GPU_TARGET_DIR) { $env:VPET_GPU_TARGET_DIR } else { "C:\vt" }
$line = ($Command | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }) -join " "
# Build tools write progress to stderr; only their exit code means failure.
$ErrorActionPreference = "Continue"
cmd /c "`"$vcvars`" >nul && set VULKAN_SDK=$sdk&& set PATH=$ninja;$sdk\Bin;%PATH%&& set CARGO_TARGET_DIR=$target&& set CMAKE_GENERATOR=Ninja&& $line"
exit $LASTEXITCODE
