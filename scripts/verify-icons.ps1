Add-Type -AssemblyName System.Drawing
$root = "c:\AI Workflow\projects\rift-tauri"
$pngs = @(
  "$root\src-tauri\icons\32x32.png",
  "$root\src-tauri\icons\128x128.png",
  "$root\src-tauri\icons\128x128@2x.png",
  "$root\src-tauri\icons\icon.png",
  "$root\src-tauri\app-icon.png",
  "$root\src\lib\assets\rift-logo.png",
  "$root\design-system\assets\rift-logo.png",
  "$root\static\favicon.png"
)
foreach ($p in $pngs) {
  $b = [System.Drawing.Bitmap]::FromFile($p)
  $w=$b.Width; $h=$b.Height
  $tl=$b.GetPixel(0,0).A; $tr=$b.GetPixel($w-1,0).A; $bl=$b.GetPixel(0,$h-1).A; $br=$b.GetPixel($w-1,$h-1).A
  $cen=$b.GetPixel([int][Math]::Floor($w/2),[int][Math]::Floor($h/2))
  $name = Split-Path $p -Leaf
  Write-Output ("{0,-22} {1}x{2} fmt={3} cornerA=[{4},{5},{6},{7}] centerA={8}" -f $name,$w,$h,$b.PixelFormat,$tl,$tr,$bl,$br,$cen.A)
  $b.Dispose()
}
# ico/icns magic
$ico = [IO.File]::ReadAllBytes("$root\src-tauri\icons\icon.ico")
Write-Output ("icon.ico header bytes: {0},{1},{2},{3}" -f $ico[0],$ico[1],$ico[2],$ico[3])
$icns = [IO.File]::ReadAllBytes("$root\src-tauri\icons\icon.icns")
$magic = [System.Text.Encoding]::ASCII.GetString($icns[0..3])
Write-Output ("icon.icns magic: $magic")
