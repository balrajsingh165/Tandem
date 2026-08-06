# Draws the Tandem desktop icon at every size Tauri bundles.
#
# The mark is a handset over a dark tile in the app's accent green: it has to read
# as "phone" at 32px in a taskbar, which rules out anything with fine detail or
# more than one colour of meaning.

Add-Type -AssemblyName System.Drawing

$OutDir = "D:\Github\Tandem\desktop\ui\src-tauri\icons"
$Tile   = [System.Drawing.Color]::FromArgb(255, 13, 16, 21)    # near-black surface
$Accent = [System.Drawing.Color]::FromArgb(255, 52, 211, 153)  # Tandem green

function New-Icon([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    # Rounded tile. The radius scales so the silhouette is identical at all sizes.
    $r = [int]($size * 0.22)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $path.AddArc(0, 0, $d, $d, 180, 90)
    $path.AddArc($size - $d, 0, $d, $d, 270, 90)
    $path.AddArc($size - $d, $size - $d, $d, $d, 0, 90)
    $path.AddArc(0, $size - $d, $d, $d, 90, 90)
    $path.CloseFigure()

    $tileBrush = New-Object System.Drawing.SolidBrush($Tile)
    $g.FillPath($tileBrush, $path)

    # Handset: a thick diagonal bar with a rounded pad at each end. Drawn with a
    # pen rather than a glyph so it does not depend on an installed font.
    $pen = New-Object System.Drawing.Pen($Accent, [single]($size * 0.105))
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

    $a = $size * 0.32
    $b = $size * 0.68
    $g.DrawLine($pen, [single]$a, [single]$b, [single]$b, [single]$a)

    $padSize = $size * 0.30
    $padBrush = New-Object System.Drawing.SolidBrush($Accent)
    $g.FillEllipse($padBrush, [single]($a - $padSize / 2), [single]($b - $padSize / 2),
                   [single]$padSize, [single]$padSize)
    $g.FillEllipse($padBrush, [single]($b - $padSize / 2), [single]($a - $padSize / 2),
                   [single]$padSize, [single]$padSize)

    # No break in the bar: a severed link is the opposite of what this app does.
    # Two nodes joined is the mark — a phone and a computer working in tandem.

    $g.Dispose()
    return $bmp
}

foreach ($spec in @(@(32, "32x32.png"), @(128, "128x128.png"), @(256, "128x128@2x.png"),
                    @(512, "icon.png"), @(30, "Square30x30Logo.png"),
                    @(44, "Square44x44Logo.png"), @(71, "Square71x71Logo.png"),
                    @(89, "Square89x89Logo.png"), @(107, "Square107x107Logo.png"),
                    @(142, "Square142x142Logo.png"), @(150, "Square150x150Logo.png"),
                    @(284, "Square284x284Logo.png"), @(310, "Square310x310Logo.png"),
                    @(50, "StoreLogo.png"))) {
    $bmp = New-Icon $spec[0]
    $path = Join-Path $OutDir $spec[1]
    if (Test-Path $path) {
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        Write-Output "wrote $($spec[1])"
    }
    $bmp.Dispose()
}

# The .ico carries several sizes so Windows picks the right one per context.
$sizes = @(16, 32, 48, 64, 128, 256)
$streams = @()
$images = @()
foreach ($s in $sizes) {
    $bmp = New-Icon $s
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $streams += $ms
    $images += $bmp
}

$ico = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($ico)
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$sizes.Count)
$offset = 6 + (16 * $sizes.Count)
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $len = [int]$streams[$i].Length
    $dim = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $w.Write([byte]$dim); $w.Write([byte]$dim)
    $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([uint16]1); $w.Write([uint16]32)
    $w.Write([uint32]$len); $w.Write([uint32]$offset)
    $offset += $len
}
foreach ($ms in $streams) { $w.Write($ms.ToArray()) }
$w.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $OutDir "icon.ico"), $ico.ToArray())
Write-Output "wrote icon.ico ($($sizes.Count) sizes)"

foreach ($bmp in $images) { $bmp.Dispose() }
foreach ($ms in $streams) { $ms.Dispose() }
