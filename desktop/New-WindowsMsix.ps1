[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Executable,
    [Parameter(Mandatory)] [string] $OutputPath,
    [Parameter(Mandatory)] [string] $MakeAppx,
    [string] $Name = 'CentralDiagnostico.Desktop',
    [string] $Publisher = 'CN=Central Diagnostico',
    [version] $Version = '0.1.0.0'
)

$ErrorActionPreference = 'Stop'
$exe = Get-Item -LiteralPath $Executable
$output = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
$staging = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString('N'))
Add-Type -AssemblyName System.Drawing
$null = New-Item -Path $staging -ItemType Directory
try {
    Copy-Item -LiteralPath $exe.FullName -Destination (Join-Path $staging 'central-desktop.exe')
    foreach ($size in 44, 50, 150) {
        $bitmap = [Drawing.Bitmap]::new($size, $size)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([Drawing.Color]::FromArgb(17, 100, 86))
            $graphics.FillRectangle([Drawing.Brushes]::White, [single]($size * .24), [single]($size * .43), [single]($size * .52), [single]($size * .14))
            $graphics.FillRectangle([Drawing.Brushes]::White, [single]($size * .43), [single]($size * .24), [single]($size * .14), [single]($size * .52))
            $bitmap.Save((Join-Path $staging "Logo$size.png"), [Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    }
    $nameXml = [Security.SecurityElement]::Escape($Name)
    $publisherXml = [Security.SecurityElement]::Escape($Publisher)
    @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="uap rescap">
  <Identity Name="$nameXml" Publisher="$publisherXml" Version="$Version" ProcessorArchitecture="x64"/>
  <Properties>
    <DisplayName>Central de Diagnóstico</DisplayName>
    <PublisherDisplayName>Central de Diagnóstico</PublisherDisplayName>
    <Logo>Logo50.png</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0"/>
  </Dependencies>
  <Resources><Resource Language="pt-BR"/></Resources>
  <Applications>
    <Application Id="Central" Executable="central-desktop.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="Central de Diagnóstico" Description="Diagnóstico local de proteção e desempenho"
                          Square44x44Logo="Logo44.png" Square150x150Logo="Logo150.png" BackgroundColor="transparent"/>
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>
</Package>
"@ | Set-Content -LiteralPath (Join-Path $staging 'AppxManifest.xml') -Encoding UTF8
    & $MakeAppx pack /d $staging /p $output /no
    if ($LASTEXITCODE -ne 0) { throw 'Falha ao empacotar com MakeAppx.' }
} finally {
    if ((Split-Path -Parent $staging) -eq ([IO.Path]::GetTempPath().TrimEnd('\'))) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
