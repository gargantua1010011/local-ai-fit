# local-ai-fit 제거: install.ps1이 만든 것(설치 폴더, 바로가기, PATH 항목, 설치된 앱 등록)을 지운다.
$ErrorActionPreference = 'SilentlyContinue'
$dest = Join-Path $env:LOCALAPPDATA 'Programs\local-ai-fit'

Get-Process local-ai-fit-gui, local-ai-fit | Stop-Process -Force
foreach ($dir in [Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop')) {
    Remove-Item (Join-Path $dir 'Local AI Fit.lnk')
}

$envKey = Get-Item 'HKCU:\Environment'
$path = $envKey.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
$parts = $path -split ';'
if ($parts -contains $dest) {
    Set-ItemProperty 'HKCU:\Environment' -Name Path -Value (($parts | Where-Object { $_ -and $_ -ne $dest }) -join ';') -Type ExpandString
    [Environment]::SetEnvironmentVariable('LOCAL_AI_FIT_TMP', '1', 'User')
    [Environment]::SetEnvironmentVariable('LOCAL_AI_FIT_TMP', $null, 'User')
}

Remove-Item 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\local-ai-fit' -Recurse
Remove-Item $dest -Recurse -Force
Write-Host 'Local AI Fit을 제거했어요.'
