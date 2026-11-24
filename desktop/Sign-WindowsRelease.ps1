[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string] $Artifact,

    [Parameter(Mandatory)]
    [string] $OutputPath,

    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string] $PublisherThumbprint,

    [ValidateSet('CurrentUser', 'LocalMachine')]
    [string] $CertificateStore = 'CurrentUser',

    [uri] $TimestampUrl = 'http://timestamp.digicert.com',

    [string] $SignTool
)

$ErrorActionPreference = 'Stop'

$source = Get-Item -LiteralPath $Artifact
if ($source -isnot [System.IO.FileInfo] -or $source.Extension -notin '.exe', '.dll', '.msi', '.msix') {
    throw 'Informe um arquivo EXE, DLL, MSI ou MSIX.'
}
$output = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
if ([IO.Path]::GetExtension($output) -ne $source.Extension) {
    throw 'A copia assinada deve manter a extensao do arquivo original.'
}
if (Test-Path -LiteralPath $output) {
    throw "O destino ja existe: $output"
}
$directory = Get-Item -LiteralPath ([IO.Path]::GetDirectoryName($output))
if ($directory -isnot [System.IO.DirectoryInfo]) {
    throw 'Informe um diretorio de destino existente.'
}
if (!$TimestampUrl.IsAbsoluteUri -or $TimestampUrl.Scheme -notin 'https', 'http') {
    throw 'Informe a URL HTTP ou HTTPS do servidor de carimbo de tempo RFC 3161.'
}

$certificate = Get-ChildItem -LiteralPath "Cert:\$CertificateStore\My" -CodeSigningCert |
    Where-Object Thumbprint -EQ $PublisherThumbprint |
    Select-Object -First 1
if (!$certificate -or !$certificate.HasPrivateKey) {
    throw "Certificado de assinatura com chave privada indisponivel em $CertificateStore\My. Conecte o token ou configure a credencial autorizada."
}
if ($certificate.Subject -eq $certificate.Issuer) {
    throw 'Use um certificado de editor emitido por uma autoridade confiavel do Windows.'
}

if (!$SignTool) {
    $sdkPath = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $sdk = Get-ChildItem -LiteralPath $sdkPath -Directory -ErrorAction SilentlyContinue |
        Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' |
        Sort-Object { [version] $_.Name } -Descending |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'x64\signtool.exe') -PathType Leaf } |
        Select-Object -First 1
    if (!$sdk) {
        throw 'SignTool x64 indisponivel. Instale o Windows SDK ou informe -SignTool.'
    }
    $SignTool = Join-Path $sdk.FullName 'x64\signtool.exe'
}
$signToolPath = (Get-Command -Name $SignTool -CommandType Application | Select-Object -First 1).Source
$storeOptions = if ($CertificateStore -eq 'LocalMachine') { @('/sm') } else { @() }
$staged = Join-Path $directory.FullName ([guid]::NewGuid().ToString('N') + $source.Extension)

try {
    [IO.File]::Copy($source.FullName, $staged)
    & $signToolPath sign /s My @storeOptions /sha1 $PublisherThumbprint /fd SHA256 /tr $TimestampUrl.AbsoluteUri /td SHA256 $staged | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw 'O SignTool retornou erro ou aviso durante a assinatura.'
    }

    $proof = & (Join-Path $PSScriptRoot 'Verify-WindowsRelease.ps1') -Artifact $staged -PublisherThumbprint $PublisherThumbprint -SignTool $signToolPath
    [IO.File]::Move($staged, $output)
    [pscustomobject]@{
        Path = $output
        SHA256 = $proof.Hash
        PublisherThumbprint = $certificate.Thumbprint
    }
} finally {
    if ([IO.File]::Exists($staged)) {
        [IO.File]::Delete($staged)
    }
}
