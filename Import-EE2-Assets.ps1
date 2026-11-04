param(
    [string]$Installation = 'C:\Program Files (x86)\GOG.com\Empire Earth II',
    [string]$Python = ''
)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    if (-not (Test-Path -LiteralPath (Join-Path $Installation 'zips\graphics.zip'))) {
        throw "Empire Earth II wurde hier nicht gefunden: $Installation"
    }
    if (-not $Python) {
        if (Test-Path -LiteralPath 'C:\Python314\python.exe') { $Python = 'C:\Python314\python.exe' }
        else { $Python = (Get-Command python -ErrorAction Stop).Source }
    }
    & $Python -c 'import PIL, numpy'
    if ($LASTEXITCODE) { throw 'Der lokale Import benötigt Pillow und NumPy in Python.' }
    & $Python 'tools\extract_ee2_textures.py' $Installation
    if ($LASTEXITCODE) { throw 'Texturimport fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_models.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Modellimport fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_lods.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der Original-Detailstufen fehlgeschlagen.' }
    & $Python 'tools\import_ee2_units.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der Originalkosten und Einheitenwerte fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_audio.py' $Installation
    if ($LASTEXITCODE) { throw 'Audioimport fehlgeschlagen.' }
    if (Test-Path -LiteralPath 'tools\extract_ee2_team_textures.py') {
        & $Python 'tools\extract_ee2_team_textures.py' --install $Installation --all-model-textures
        if ($LASTEXITCODE) { throw 'Import der Spielerfarben fehlgeschlagen.' }
    }
    Write-Host 'Lokale Originalgrafiken und Originalklänge wurden importiert.'
    Write-Host 'Die Installation wurde nur gelesen. Ausgabe: docs\local-original'
} finally { Pop-Location }
