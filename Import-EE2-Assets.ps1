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
    # Scale bindings need the model inventory; Farm composites then use those
    # scale bindings. Defer Farm during the first geometry pass to avoid a
    # circular prerequisite on a fresh import.
    & $Python 'tools\extract_ee2_models.py' --install $Installation --defer-farm
    if ($LASTEXITCODE) { throw 'Modellimport fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_lods.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der Original-Detailstufen fehlgeschlagen.' }
    & $Python 'tools\import_ee2_visual_scales.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der nativen Modellmaßstäbe fehlgeschlagen.' }
    & $Python 'tools\import_ee2_farm_composites.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der originalen Farm-Teilfelder fehlgeschlagen.' }
    & $Python 'tools\import_ee2_ground_profile.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der nativen Gebäudeböden fehlgeschlagen.' }
    & $Python 'tools\import_ee2_ground_seasons.py'
    if ($LASTEXITCODE) { throw 'Import der saisonalen Gebäudeböden fehlgeschlagen.' }
    & $Python 'tools\import_ee2_native_terrain.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der nativen Gelände-Texturen fehlgeschlagen.' }
    & $Python 'tools\import_ee2_native_water.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der originalen Wasser-Materialien fehlgeschlagen.' }
    & $Python 'tools\import_ee2_units.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der Originalkosten und Einheitenwerte fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_audio.py' $Installation
    if ($LASTEXITCODE) { throw 'Audioimport fehlgeschlagen.' }
    if (Test-Path -LiteralPath 'tools\extract_ee2_team_textures.py') {
        & $Python 'tools\extract_ee2_team_textures.py' --install $Installation --all-model-textures
        if ($LASTEXITCODE) { throw 'Import der Spielerfarben fehlgeschlagen.' }
    }
    & $Python 'tools\import_ee2_model_seasons.py' --install $Installation
    if ($LASTEXITCODE) { throw 'Import der saisonalen Modelltexturen fehlgeschlagen.' }
    & $Python 'tools\extract_ee2_hud.py' --install $Installation --enhanced
    if ($LASTEXITCODE) { throw 'Import der hochauflösenden Originaloberfläche fehlgeschlagen.' }
    Write-Host 'Lokale Originalgrafiken und Originalklänge wurden importiert.'
    Write-Host 'Die Installation wurde nur gelesen. Ausgabe: docs\local-original'
} finally { Pop-Location }
