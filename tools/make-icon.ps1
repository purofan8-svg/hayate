Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Drawing.Drawing2D

$S = 256
$bmp = New-Object System.Drawing.Bitmap($S, $S)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::Transparent)

# фон: скруглённый квадрат с градиентом
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$r = 56
$path.AddArc(8, 8, $r, $r, 180, 90)
$path.AddArc($S - 8 - $r, 8, $r, $r, 270, 90)
$path.AddArc($S - 8 - $r, $S - 8 - $r, $r, $r, 0, 90)
$path.AddArc(8, $S - 8 - $r, $r, $r, 90, 90)
$path.CloseFigure()

$rect = New-Object System.Drawing.Rectangle(8, 8, $S - 16, $S - 16)
$brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 44, 46, 58),
    [System.Drawing.Color]::FromArgb(255, 18, 18, 24),
    90.0)
$g.FillPath($brush, $path)

# ветер: три дуги
$accent = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 76, 194, 255), 14)
$accent.StartCap = 'Round'
$accent.EndCap = 'Round'
$accent2 = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(210, 76, 194, 255), 12)
$accent2.StartCap = 'Round'
$accent2.EndCap = 'Round'

$g.DrawArc($accent, 52, 70, 150, 70, 200, 140)
$g.DrawArc($accent2, 40, 118, 130, 60, 200, 140)
$g.DrawArc($accent2, 58, 166, 110, 50, 200, 140)

# лупа поверх ветра
$lens = New-Object System.Drawing.Pen([System.Drawing.Color]::White, 12)
$lens.StartCap = 'Round'
$lens.EndCap = 'Round'
$g.DrawEllipse($lens, 138, 132, 62, 62)
$g.DrawLine($lens, 194, 188, 216, 210)

$out = "C:\Users\Phiron\IdeaProjects\hayate\assets\icon.png"
New-Item -ItemType Directory -Force -Path (Split-Path $out) | Out-Null
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Output "saved $out"