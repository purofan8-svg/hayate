param([string]$Version = "0.1.0", [string]$Root = (Split-Path $PSScriptRoot -Parent))
$ErrorActionPreference = "Stop"
$root = $Root
$dist = Join-Path $root "dist"
$stage = Join-Path $dist "hayate-$Version-win64"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item (Join-Path $root "target\release\hayate.exe") $stage
Copy-Item (Join-Path $root "target\release\hayate-setup.exe") $stage
Copy-Item (Join-Path $root "LICENSE") $stage -ErrorAction SilentlyContinue

$readme = @"
hayate $Version — мгновенный поиск файлов
========================================

УСТАНОВКА (обычный способ)
  1. Распакуй этот архив целиком (не запускай изнутри zip!).
  2. Запусти hayate-setup.exe
  3. Готово: ярлык появился в меню Пуск и на рабочем столе.
     Удаление: Параметры → Приложения → hayate.

ПОРТАТИВНЫЙ СПОСОБ
  Просто запусти hayate.exe из этой папки — без установки.

ПЕРВЫЙ ЗАПУСК
  Установщик запустит окно. Индекса ещё нет — нажми «Построить индекс»,
  это один раз займёт несколько минут (зависит от дисков).
  Дальше поиск идёт за миллисекунды, индекс обновляется командой
  hayate index (или сам, когда появится USN-обновление).

КОМАНДНАЯ СТРОКА
  hayate              — окно поиска
  hayate index        — перестроить индекс
  hayate find "мод"   — поиск в консоли
  hayate tui          — поиск в терминале

НАСТРОЙКИ
  Шестерёнка справа сверху или Ctrl+,
  Хранятся в %LOCALAPPDATA%\hayate\settings.json
"@
Set-Content -Path (Join-Path $stage "README.txt") -Value $readme -Encoding UTF8

$zip = Join-Path $dist "hayate-$Version-win64.zip"
Remove-Item -Force $zip -ErrorAction SilentlyContinue
Compress-Archive -Path "$stage\*" -DestinationPath $zip -CompressionLevel Optimal
Write-Output ("готово: " + $zip + " (" + [math]::Round((Get-Item $zip).Length / 1MB, 1) + " МБ)")