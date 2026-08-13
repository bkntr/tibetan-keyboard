$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$workspace = Split-Path -Parent $PSScriptRoot
$assetDir = Join-Path $workspace 'assets'
$previewDir = Join-Path $assetDir 'preview'
New-Item -ItemType Directory -Path $assetDir -Force | Out-Null
New-Item -ItemType Directory -Path $previewDir -Force | Out-Null

# U+0F00 TIBETAN SYLLABLE OM. Constructed by code point so Windows
# PowerShell's source-file encoding cannot change the glyph.
$om = [string][char]0x0F00

function New-TibetanOmIcon {
    param(
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][System.Drawing.Color]$Background
    )

    $size = 256
    $bitmap = [System.Drawing.Bitmap]::new(
        $size,
        $size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $glyphPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $matrix = [System.Drawing.Drawing2D.Matrix]::new()
    $family = [System.Drawing.FontFamily]::new('Microsoft Himalaya')
    $backgroundBrush = [System.Drawing.SolidBrush]::new($Background)
    $glyphBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)

    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

        # The badge itself uses virtually the full icon canvas.
        $graphics.FillEllipse($backgroundBrush, 3, 3, 250, 250)

        # Convert the authoritative font glyph to an outline, then fit its
        # measured bounds rather than relying on DrawString's large line box.
        $glyphPath.AddString(
            $om,
            $family,
            [int][System.Drawing.FontStyle]::Regular,
            220,
            [System.Drawing.PointF]::new(0, 0),
            [System.Drawing.StringFormat]::GenericTypographic
        )
        $bounds = $glyphPath.GetBounds()
        $targetWidth = 196.0
        $targetHeight = 196.0
        $scale = [Math]::Min($targetWidth / $bounds.Width, $targetHeight / $bounds.Height)
        $matrix.Translate(-$bounds.X, -$bounds.Y)
        $matrix.Scale($scale, $scale, [System.Drawing.Drawing2D.MatrixOrder]::Append)
        $scaledWidth = $bounds.Width * $scale
        $scaledHeight = $bounds.Height * $scale
        $matrix.Translate(
            (256.0 - $scaledWidth) / 2.0,
            (256.0 - $scaledHeight) / 2.0,
            [System.Drawing.Drawing2D.MatrixOrder]::Append
        )
        $glyphPath.Transform($matrix)
        $graphics.FillPath($glyphBrush, $glyphPath)

        $bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $glyphBrush.Dispose()
        $backgroundBrush.Dispose()
        $family.Dispose()
        $matrix.Dispose()
        $glyphPath.Dispose()
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function New-Preview {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination,
        [Parameter(Mandatory = $true)][int]$Size
    )

    $sourceImage = [System.Drawing.Image]::FromFile($Source)
    $preview = [System.Drawing.Bitmap]::new(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($preview)
    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.DrawImage($sourceImage, 0, 0, $Size, $Size)
        $preview.Save($Destination, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $preview.Dispose()
        $sourceImage.Dispose()
    }
}

$enabled = Join-Path $assetDir 'om-enabled.png'
$disabled = Join-Path $assetDir 'om-disabled.png'

New-TibetanOmIcon `
    -OutputPath $enabled `
    -Background ([System.Drawing.Color]::FromArgb(255, 0, 150, 92))
New-TibetanOmIcon `
    -OutputPath $disabled `
    -Background ([System.Drawing.Color]::FromArgb(255, 105, 115, 124))

New-Preview -Source $enabled -Destination (Join-Path $previewDir 'om-enabled-16.png') -Size 16
New-Preview -Source $disabled -Destination (Join-Path $previewDir 'om-disabled-16.png') -Size 16
New-Preview -Source $enabled -Destination (Join-Path $previewDir 'om-enabled-32.png') -Size 32
New-Preview -Source $disabled -Destination (Join-Path $previewDir 'om-disabled-32.png') -Size 32
New-Preview -Source $enabled -Destination (Join-Path $assetDir 'om-enabled-tray.png') -Size 32
New-Preview -Source $disabled -Destination (Join-Path $assetDir 'om-disabled-tray.png') -Size 32

Write-Output "Rendered exact U+0F00 Tibetan OM tray assets in $assetDir"
