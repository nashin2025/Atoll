$icon = New-Object System.Drawing.Icon('wallet/gui/src-tauri/icons/icon.ico', 256, 256)
$bitmap = $icon.ToBitmap()
$bitmap.Save('wallet/gui/src-tauri/icons/icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
$icon.Dispose()
Write-Host 'Created icon.png'
