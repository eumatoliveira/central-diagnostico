[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string[]] $Artifact,

    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string] $PublisherThumbprint,

    [string] $SignTool = 'signtool.exe'
)

$ErrorActionPreference = 'Stop'
$verified = foreach ($path in $Artifact) {
    $file = Get-Item -LiteralPath $path
    if ($file -isnot [System.IO.FileInfo]) {
        throw "Informe o caminho de um arquivo: $path"
    }

    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.Thumbprint -ne $PublisherThumbprint) {
        throw "Assinatura invalida ou editor diferente do esperado: $($file.Name)"
    }

    & $SignTool verify /pa /all /tw /v $file.FullName | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "A verificacao do Windows SDK retornou erro ou aviso: $($file.Name)"
    }

    Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256
}

$verified
