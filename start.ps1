$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
if ($args.Count -eq 0) {
    & "C:\Users\sandr\.cargo\bin\cargo.exe" run -p formcraft-mcp -- --help
} else {
    & "C:\Users\sandr\.cargo\bin\cargo.exe" run -p formcraft-mcp -- @args
}
