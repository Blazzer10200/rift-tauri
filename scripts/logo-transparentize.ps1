param(
  [string]$Src = "C:\Users\BLAZZER\Downloads\RIFT LOGO.png",
  [string]$Out = "C:\Users\BLAZZER\Downloads\RIFT_nobg.png",
  [int]$Thresh = 16   # max channel value still treated as "corner black"
)
# Flood-fill transparency from the 4 corners over near-black pixels only, so the
# squircle's interior dark pixels are never touched. Produces a 32bpp ARGB PNG
# with transparent corners (the cornerA=0 the app-icon pipeline expects).
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = "Stop"

$img = [System.Drawing.Image]::FromFile($Src)
$w = $img.Width; $h = $img.Height
$bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.DrawImage($img, 0, 0, $w, $h)
$g.Dispose()

# Lock bits for fast raw access (BGRA, stride-aligned).
$rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
$data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadWrite, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$stride = $data.Stride
$bytes = New-Object byte[] ($stride * $h)
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)

$visited = New-Object 'bool[]' ($w * $h)
$stack = New-Object System.Collections.Generic.Stack[int]
# Seed the entire border ring (top/bottom rows + left/right columns), not just
# the 4 corners — a single non-black edge pixel can otherwise strand a corner.
for ($x = 0; $x -lt $w; $x++) { $stack.Push($x); $stack.Push($w*($h-1) + $x) }
for ($y = 0; $y -lt $h; $y++) { $stack.Push($y*$w); $stack.Push($y*$w + ($w-1)) }

$cleared = 0
while ($stack.Count -gt 0) {
  $idx = $stack.Pop()
  if ($idx -lt 0 -or $idx -ge ($w*$h)) { continue }
  if ($visited[$idx]) { continue }
  $visited[$idx] = $true
  $x = $idx % $w; $y = [int][Math]::Floor($idx / $w)
  $o = $y * $stride + $x * 4
  if ($o + 3 -ge $bytes.Length) { continue }
  # BGRA order
  $b = $bytes[$o]; $gr = $bytes[$o+1]; $r = $bytes[$o+2]
  if ($r -gt $Thresh -or $gr -gt $Thresh -or $b -gt $Thresh) { continue }
  $bytes[$o+3] = 0   # clear alpha
  $cleared++
  if ($x -gt 0)      { $stack.Push($idx - 1) }
  if ($x -lt $w-1)   { $stack.Push($idx + 1) }
  if ($y -gt 0)      { $stack.Push($idx - $w) }
  if ($y -lt $h-1)   { $stack.Push($idx + $w) }
}

[System.Runtime.InteropServices.Marshal]::Copy($bytes, 0, $data.Scan0, $bytes.Length)
$bmp.UnlockBits($data)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose(); $img.Dispose()
Write-Output ("wrote {0} ({1}x{1}); cleared {2} corner px to transparent" -f $Out, $w, $cleared)
