# local-ai-fit 설치: 빌드된 exe를 사용자 프로그램 폴더에 복사하고 시작 메뉴·바탕화면 바로가기,
# PATH(터미널에서 local-ai-fit 명령), "설치된 앱" 목록(제거 버튼)을 등록한다. 관리자 권한 필요 없음.
# 실행: powershell -ExecutionPolicy Bypass -File install\install.ps1 [-NoDesktop]
param([switch]$NoDesktop)
$ErrorActionPreference = 'Stop'

$proj = Split-Path $PSScriptRoot -Parent
$src = Join-Path $proj 'target\release'
$dest = Join-Path $env:LOCALAPPDATA 'Programs\local-ai-fit'
$exes = 'local-ai-fit-gui.exe', 'local-ai-fit.exe'
foreach ($e in $exes) {
    if (-not (Test-Path (Join-Path $src $e))) {
        Write-Host "$e 이 없어요. 먼저 빌드해 주세요: cargo build --release" -ForegroundColor Red
        exit 1
    }
}
$version = (Select-String -Path (Join-Path $proj 'Cargo.toml') -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value

# 실행 중이면 파일이 잠겨 있으니 닫는다
Get-Process local-ai-fit-gui, local-ai-fit -ErrorAction SilentlyContinue | Stop-Process -Force
New-Item -ItemType Directory -Force $dest | Out-Null
foreach ($e in $exes) { Copy-Item (Join-Path $src $e) $dest -Force }
Copy-Item (Join-Path $PSScriptRoot 'uninstall.ps1') $dest -Force
# --update로 받아둔 목록이 있으면 같이 옮긴다(설치 폴더에 이미 있으면 그대로 둠)
$list = Join-Path $src 'models.json'
if ((Test-Path $list) -and -not (Test-Path (Join-Path $dest 'models.json'))) { Copy-Item $list $dest }

# 바로가기
$gui = Join-Path $dest 'local-ai-fit-gui.exe'
$shell = New-Object -ComObject WScript.Shell
$links = @([Environment]::GetFolderPath('Programs'))
if (-not $NoDesktop) { $links += [Environment]::GetFolderPath('Desktop') }
foreach ($dir in $links) {
    $lnk = $shell.CreateShortcut((Join-Path $dir 'Local AI Fit.lnk'))
    $lnk.TargetPath = $gui
    $lnk.WorkingDirectory = $dest
    $lnk.IconLocation = "$gui,0"
    $lnk.Description = '내 PC에서 돌릴 수 있는 로컬 AI 모델 보기'
    $lnk.Save()
}

# 사용자 PATH에 추가(%VAR% 형태가 풀리지 않게 레지스트리 원문 그대로 다룬다)
$envKey = Get-Item 'HKCU:\Environment'
$path = $envKey.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
if (($path -split ';') -notcontains $dest) {
    $new = (($path.TrimEnd(';'), $dest) | Where-Object { $_ }) -join ';'
    Set-ItemProperty 'HKCU:\Environment' -Name Path -Value $new -Type ExpandString
    # 열려 있는 탐색기에 환경 변수 변경을 알린다(새로 여는 터미널부터 적용)
    [Environment]::SetEnvironmentVariable('LOCAL_AI_FIT_TMP', '1', 'User')
    [Environment]::SetEnvironmentVariable('LOCAL_AI_FIT_TMP', $null, 'User')
}

# 설정 > 앱 > 설치된 앱 에 등록
$uninst = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\local-ai-fit'
New-Item -Force $uninst | Out-Null
$props = @{
    DisplayName     = 'Local AI Fit'
    DisplayVersion  = $version
    Publisher       = 'local-ai-fit'
    DisplayIcon     = "$gui,0"
    InstallLocation = $dest
    UninstallString = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$dest\uninstall.ps1`""
}
foreach ($k in $props.Keys) { Set-ItemProperty $uninst -Name $k -Value $props[$k] }
Set-ItemProperty $uninst -Name NoModify -Value 1 -Type DWord
Set-ItemProperty $uninst -Name NoRepair -Value 1 -Type DWord
$size = (Get-ChildItem $dest | Measure-Object Length -Sum).Sum / 1KB
Set-ItemProperty $uninst -Name EstimatedSize -Value ([int]$size) -Type DWord

Write-Host "설치했어요 (버전 $version): $dest" -ForegroundColor Green
Write-Host "- 시작 메뉴$(if (-not $NoDesktop) { '와 바탕화면' })에서 'Local AI Fit'을 찾아 실행하세요."
Write-Host "- 새로 연 터미널에서는 'local-ai-fit' 명령도 쓸 수 있어요."
Write-Host "- 지우려면 설정 > 앱 > 설치된 앱에서 'Local AI Fit'을 제거하세요."
