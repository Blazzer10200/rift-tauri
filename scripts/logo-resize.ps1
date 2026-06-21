param(
  [string]$Src = "C:\Users\BLAZZER\Downloads\RIFT_nobg.png"
)
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = "Stop"

function Save-Square {
  param([System.Drawing.Image]$Img, [int]$Size, [string]$Out)
  $bmp = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.Color]::Transparent)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  # Fit the source square into the canvas, centered, preserving aspect.
  $scale = [Math]::Min($Size / $Img.Width, $Size / $Img.Height)
  $w = [int]([Math]::Round($Img.Width * $scale))
  $h = [int]([Math]::Round($Img.Height * $scale))
  $x = [int](($Size - $w) / 2)
  $y = [int](($Size - $h) / 2)
  $g.DrawImage($Img, $x, $y, $w, $h)
  $g.Dispose()
  $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  Write-Output ("wrote {0} ({1}x{1})" -f $Out, $Size)
}

$img = [System.Drawing.Image]::FromFile($Src)
$root = "c:\AI Workflow\projects\rift-tauri"

Save-Square $img 1024 "$root\src-tauri\app-icon.png"
Save-Square $img 512  "$root\src\lib\assets\rift-logo.png"
Save-Square $img 512  "$root\design-system\assets\rift-logo.png"
Save-Square $img 256  "$root\static\favicon.png"

$img.Dispose()
Write-Output "DONE"
