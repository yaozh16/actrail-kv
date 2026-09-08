#Requires -Version 5.1
# Full-chain deploy/manager for Windows (agent-cassette + actrail-kv).
# Mirrors anthropic-agent-chain.sh. This script does NOT launch an agent.
#
# Commands:
#   --init    start/reuse receiver + agent-cassette, print agent env lines
#   --env     print ANTHROPIC_* env lines for your current PowerShell session
#   --health  check healthz / receiver port / ndjson / OpenAI / Anthropic endpoints
#   --report  run actrail-kv analyze + report on existing ndjson
#   --stop    stop processes started with the cassette conf / receiver output

$ErrorActionPreference = 'Stop'
# HttpClient lives in System.Net.Http, which PowerShell 5.1 does not load by
# default; without this the OpenAI/Anthropic probes in --health crash.
Add-Type -AssemblyName System.Net.Http

function Get-EnvOrDefault([string]$name, [string]$fallback) {
    $value = [Environment]::GetEnvironmentVariable($name)
    if ($value) { return $value }
    return $fallback
}

$KVRootDefault = Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$KVRoot     = Get-EnvOrDefault 'KV_ROOT'     $KVRootDefault
$CassetteRepo = Get-EnvOrDefault 'CASSETTE_REPO' (Join-Path $KVRoot 'agent-cassette')
$ActrailRepo  = Get-EnvOrDefault 'ACTRAIL_REPO'  (Join-Path $KVRoot 'actrail-kv')
$DocRoot      = Get-EnvOrDefault 'DOC_ROOT'      (Join-Path $KVRoot 'doc')
$CassetteConf = Get-EnvOrDefault 'CASSETTE_CONF' (Join-Path $KVRoot 'conf\agent-cassette.toml')

$CassettePort = [Environment]::GetEnvironmentVariable('CASSETTE_PORT')
$ReceiverPort = [Environment]::GetEnvironmentVariable('RECEIVER_PORT')
$ReceiverOut  = [Environment]::GetEnvironmentVariable('RECEIVER_OUT')

$RunName = Get-EnvOrDefault 'RUN_NAME' ("anthropic-agent-" + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$StateFile = Join-Path (Join-Path $KVRoot 'run') '.anthropic-agent-current.env'

if (-not $ReceiverOut) {
    $ReceiverOut = Join-Path (Join-Path $KVRoot 'run') ("requests-" + $RunName + '.ndjson')
}

$CassetteExe = Join-Path $CassetteRepo 'target\release\agent-cassette.exe'
$ReceiverExe = Join-Path $ActrailRepo  'target\release\actrail-kv-receiver.exe'
$AnalyzeExe  = Join-Path $ActrailRepo  'target\release\actrail-kv-analyze.exe'
$ReportExe   = Join-Path $ActrailRepo  'target\release\actrail-kv-report.exe'

function Log([string]$message) {
    Write-Host ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $message)
}

function Fail([string]$message) {
    Write-Host ("FAIL: " + $message) -ForegroundColor Red
    exit 1
}

function Pass([string]$message) {
    Write-Host ("PASS: " + $message) -ForegroundColor Green
}

function Show-Usage {
    @"
Usage: powershell -ExecutionPolicy Bypass -File fullchain-deploy-windows.ps1 <command>

Commands (with or without leading "--", same as the Linux script):
  init      Start/reuse receiver + agent-cassette and print agent env lines
  env       Print ANTHROPIC_* env lines for the current session
  health    Check healthz, receiver port, ndjson, OpenAI and Anthropic endpoints
  report    Run actrail-kv analyze + report on existing ndjson
  stop      Stop processes started with the cassette conf / receiver output
  -h|--help Show this help

Environment overrides: KV_ROOT, CASSETTE_REPO, ACTRAIL_REPO, DOC_ROOT,
                       CASSETTE_CONF, CASSETTE_PORT, RECEIVER_PORT, RECEIVER_OUT
"@
    exit 0
}

function Get-TomlValue([string]$path, [string]$section, [string]$key) {
    if (-not (Test-Path $path)) { return '' }
    $current = ''
    # -Encoding UTF8 is required: PowerShell 5.1 reads non-BOM files as ANSI/GBK,
    # and multi-byte UTF-8 comment tails can swallow the following LF, merging
    # lines so section headers like [gateway.transport] stop being detected.
    foreach ($line in Get-Content -Path $path -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[(.+)\]$') {
            $current = $Matches[1].Trim()
            continue
        }
        if ($current -ne $section) { continue }
        if ($trimmed -match ('^' + [regex]::Escape($key) + '\s*=\s*"?(.*?)"?\s*$')) {
            return $Matches[1]
        }
    }
    return ''
}

function Get-PortFromValue([string]$value) {
    if ($value -match ':(\d+)\s*$') { return $Matches[1] }
    return ''
}

function Test-PortOpen([int]$port) {
    try {
        $client = New-Object Net.Sockets.TcpClient
        $client.Connect('127.0.0.1', $port)
        $client.Close()
        return $true
    } catch {
        return $false
    }
}

function Wait-Port([int]$port, [int]$tries = 60) {
    for ($i = 0; $i -lt $tries; $i++) {
        if (Test-PortOpen $port) { return $true }
        Start-Sleep -Seconds 1
    }
    return $false
}

function Resolve-Ports {
    # Assignments must go through $script: — a plain assignment inside a
    # function creates a local copy and callers keep seeing the old (empty) value.
    if (-not $script:CassettePort) {
        $listen = Get-TomlValue $CassetteConf 'gateway.transport' 'listen_addr'
        if ($listen) { $script:CassettePort = Get-PortFromValue $listen }
    }
    if (-not $script:ReceiverPort) {
        $url = Get-TomlValue $CassetteConf 'recorder.actrail' 'receiver_url'
        if ($url) { $script:ReceiverPort = Get-PortFromValue $url }
    }
    if (-not $script:CassettePort) { $script:CassettePort = '18194' }
    if (-not $script:ReceiverPort) { $script:ReceiverPort = '8100' }
    Log ("ports: cassette=" + $script:CassettePort + " receiver=" + $script:ReceiverPort)
}

function Save-State {
    $dir = Split-Path -Parent $StateFile
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    @(
        "RUN_NAME=$RunName",
        "RECEIVER_OUT=$ReceiverOut",
        "CASSETTE_CONF=$CassetteConf",
        "CASSETTE_PORT=$CassettePort",
        "RECEIVER_PORT=$ReceiverPort"
    ) | Set-Content -Path $StateFile -Encoding UTF8
}

function Load-State {
    if (-not (Test-Path $StateFile)) { return $false }
    foreach ($line in Get-Content -Path $StateFile -Encoding UTF8) {
        if ($line -match '^([^=]+)=(.*)$') {
            switch ($Matches[1]) {
                'RUN_NAME'       { $script:RunName = $Matches[2] }
                'RECEIVER_OUT'   { $script:ReceiverOut = $Matches[2] }
                'CASSETTE_CONF'  { $script:CassetteConf = $Matches[2] }
                'CASSETTE_PORT'  { $script:CassettePort = $Matches[2] }
                'RECEIVER_PORT'  { $script:ReceiverPort = $Matches[2] }
            }
        }
    }
    return $true
}

function Get-RunningReceiverOutput {
    $proc = Get-CimInstance Win32_Process -Filter "Name = 'actrail-kv-receiver.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -match '--output' } |
        Select-Object -First 1
    if ($proc -and $proc.CommandLine -match '--output[= ]"?([^"\s]+)"?') { return $Matches[1] }
    return ''
}

function Invoke-CmdInit {
    Resolve-Ports
    if (-not (Test-Path $CassetteConf)) { Fail ("cassette config not found: " + $CassetteConf + " (set CASSETTE_CONF)") }
    $runDir = Join-Path $KVRoot 'run'
    if (-not (Test-Path $runDir)) { New-Item -ItemType Directory -Force -Path $runDir | Out-Null }
    $receiverLog = Join-Path $runDir ("receiver-" + $RunName + '.log')
    $cassetteLog = Join-Path $runDir ("agent-cassette-" + $RunName + '.log')

    if (Test-PortOpen ([int]$ReceiverPort)) {
        Log ("receiver port " + $ReceiverPort + " in use: reuse existing receiver")
        # The reused receiver may write to a different --output file than this
        # run's default; resolve the real path so health/report point at it.
        $runningOut = Get-RunningReceiverOutput
        if ($runningOut) {
            $script:ReceiverOut = $runningOut
            Log ("receiver ndjson (from running process): " + $runningOut)
        }
    } else {
        Log ("start receiver :" + $ReceiverPort)
        if (-not (Test-Path $ReceiverExe)) { Fail ("receiver binary missing: " + $ReceiverExe) }
        $proc = Start-Process -FilePath $ReceiverExe `
            -ArgumentList @('--listen', ('127.0.0.1:' + $ReceiverPort), '--output', $ReceiverOut) `
            -RedirectStandardOutput $receiverLog -RedirectStandardError ($receiverLog + '.err') `
            -WindowStyle Hidden -PassThru
        if (-not (Wait-Port ([int]$ReceiverPort))) { Fail ("receiver did not listen; see " + $receiverLog) }
        Pass 'receiver started'
    }

    if (Test-PortOpen ([int]$CassettePort)) {
        Log ("cassette port " + $CassettePort + " in use: reuse existing instance")
    } else {
        Log ("start agent-cassette :" + $CassettePort)
        if (-not (Test-Path $CassetteExe)) { Fail ("agent-cassette binary missing: " + $CassetteExe) }
        # agent-cassette reads the provider key at startup; fail fast instead of
        # waiting for the port timeout.
        $keyEnv = Get-TomlValue $CassetteConf 'proxy.provider' 'api_key_env'
        if ($keyEnv -and -not [Environment]::GetEnvironmentVariable($keyEnv)) {
            Fail ("environment variable " + $keyEnv + " (from [proxy.provider].api_key_env) is not set; set it before starting agent-cassette")
        }
        $proc = Start-Process -FilePath $CassetteExe `
            -ArgumentList @('-c', $CassetteConf, 'start') `
            -RedirectStandardOutput $cassetteLog -RedirectStandardError ($cassetteLog + '.err') `
            -WindowStyle Hidden -PassThru
        if (-not (Wait-Port ([int]$CassettePort))) { Fail ("agent-cassette did not listen; see " + $cassetteLog) }
        Pass 'agent-cassette started'
    }

    # Persist even when everything was reused, so health/report/stop reference
    # the resolved receiver output file.
    Save-State

    $token = if ($env:ANTHROPIC_API_KEY) { $env:ANTHROPIC_API_KEY } else { 'cassette-proxy' }
    $model = if ($env:ANTHROPIC_MODEL) { $env:ANTHROPIC_MODEL } else { Get-TomlValue $CassetteConf 'proxy.provider' 'model' }
    if (-not $model) { $model = 'default' }

    @"
========================================================
recording chain ready: run=$RunName
  agent-cassette: http://127.0.0.1:$CassettePort
  receiver ndjson: $ReceiverOut

Point the agent at cassette (set these in the launching terminal, then start the agent):
  `$env:ANTHROPIC_BASE_URL = "http://127.0.0.1:$CassettePort"
  `$env:ANTHROPIC_API_KEY   = "$token"
  `$env:ANTHROPIC_MODEL     = "$model"

Shortcut: iex (& '$PSCommandPath' --env)

After chatting run:
  powershell -File '$PSCommandPath' --report
  powershell -File '$PSCommandPath' --stop
========================================================
"@
}

function Invoke-CmdEnv {
    if (-not (Load-State)) { Resolve-Ports }
    $token = if ($env:ANTHROPIC_API_KEY) { $env:ANTHROPIC_API_KEY } else { 'cassette-proxy' }
    $model = if ($env:ANTHROPIC_MODEL) { $env:ANTHROPIC_MODEL } else { Get-TomlValue $CassetteConf 'proxy.provider' 'model' }
    if (-not $model) { $model = 'default' }
    @"
# run: iex (& '$PSCommandPath' --env)
`$env:ANTHROPIC_BASE_URL = "http://127.0.0.1:$CassettePort"
`$env:ANTHROPIC_API_KEY   = "$token"
`$env:ANTHROPIC_MODEL     = "$model"
"@
}

function Invoke-HttpProbe([string]$url, [string]$json, [hashtable]$headers = @{}) {
    $client = New-Object Net.Http.HttpClient
    $client.Timeout = [TimeSpan]::FromSeconds(90)
    $content = New-Object Net.Http.StringContent($json, [Text.Encoding]::UTF8, 'application/json')
    foreach ($key in $headers.Keys) { $client.DefaultRequestHeaders.TryAddWithoutValidation($key, $headers[$key]) | Out-Null }
    try {
        $response = $client.PostAsync($url, $content).GetAwaiter().GetResult()
        return [int]$response.StatusCode
    } catch {
        return 0
    }
}

function Invoke-CmdHealth {
    if (-not (Load-State)) { Resolve-Ports }
    $ok = $true
    try {
        Invoke-WebRequest -Uri ("http://127.0.0.1:" + $CassettePort + '/healthz') -UseBasicParsing -TimeoutSec 3 | Out-Null
        Pass ("cassette healthz ok (127.0.0.1:" + $CassettePort + ")")
    } catch {
        Write-Host ("FAIL: cassette healthz unreachable (127.0.0.1:" + $CassettePort + ")") -ForegroundColor Red
        $ok = $false
    }
    if (Test-PortOpen ([int]$ReceiverPort)) {
        Pass ("receiver port reachable (127.0.0.1:" + $ReceiverPort + ")")
    } else {
        Write-Host ("FAIL: receiver port unreachable (127.0.0.1:" + $ReceiverPort + ")") -ForegroundColor Red
        $ok = $false
    }
    if (Test-Path $ReceiverOut) {
        try {
            $lines = (Get-Content $ReceiverOut | Measure-Object -Line).Lines
            Log ("ndjson: " + $ReceiverOut + " (" + $lines + " lines)")
        } catch {
            # Windows 使用 FILE_SHARE_READ 排除写方、不阻塞读方，不再使用
            # LockFileEx；此处即使出现偶发读错误也仅提示，不作为健康失败。
            Log ("ndjson: " + $ReceiverOut + " (present; read skipped: " + $_.Exception.Message + ")")
        }
    } else {
        Log ("ndjson: not found " + $ReceiverOut)
    }
    $model = if ($env:ANTHROPIC_MODEL) { $env:ANTHROPIC_MODEL } else { Get-TomlValue $CassetteConf 'proxy.provider' 'model' }
    if (-not $model) { $model = 'default' }

    $openai = Invoke-HttpProbe `
        ("http://127.0.0.1:" + $CassettePort + '/v1/chat/completions') `
        ('{"model":"' + $model + '","max_tokens":8,"messages":[{"role":"user","content":"ping"}],"stream":false}')
    if ($openai -eq 200) { Pass 'OpenAI /v1/chat/completions HTTP 200' } else { Write-Host ("FAIL: OpenAI endpoint HTTP " + $openai) -ForegroundColor Red; $ok = $false }

    $token = if ($env:ANTHROPIC_API_KEY) { $env:ANTHROPIC_API_KEY } else { 'cassette-proxy' }
    $anthropic = Invoke-HttpProbe `
        ("http://127.0.0.1:" + $CassettePort + '/v1/messages') `
        ('{"model":"' + $model + '","max_tokens":8,"messages":[{"role":"user","content":"ping"}]}') `
        @{ 'anthropic-version' = '2023-06-01'; 'x-api-key' = $token }
    if ($anthropic -eq 200) { Pass 'Anthropic /v1/messages HTTP 200' } else { Write-Host ("FAIL: Anthropic endpoint HTTP " + $anthropic) -ForegroundColor Red; $ok = $false }

    if ($ok) { Pass 'HEALTH: PASS'; exit 0 }
    Write-Host 'HEALTH: FAIL' -ForegroundColor Red
    exit 1
}

function Invoke-CmdReport {
    # Prefer the state file, then the newest requests-*.ndjson in run\, so --report
    # also works from a fresh terminal (RunName changes on every invocation).
    if (-not (Test-Path $ReceiverOut)) { $null = Load-State }
    if (-not (Test-Path $ReceiverOut)) {
        $newest = Get-ChildItem -Path (Join-Path $KVRoot 'run') -Filter 'requests-*.ndjson' -File -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($newest) { $script:ReceiverOut = $newest.FullName }
    }
    if (-not (Test-Path $ReceiverOut)) {
        Fail ("ndjson not found: " + $ReceiverOut + " (set RECEIVER_OUT or run --init first)")
    }
    $budget = Get-EnvOrDefault 'BUDGET_CONF' ''
    if (-not $budget) {
        $example = Join-Path $ActrailRepo 'examples\analyze.config.example.json'
        if (Test-Path $example) { $budget = $example }
    }
    if (-not $budget) { Fail 'analyze config not found (set BUDGET_CONF)' }
    $outName = 'actrail-kv-run-anthropic-agent-report-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
    $outDir = Join-Path $DocRoot $outName
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    Log ('analyze ' + $ReceiverOut)
    & $AnalyzeExe --config $budget --input $ReceiverOut --output (Join-Path $outDir 'analysis.json')
    if ($LASTEXITCODE -ne 0) { Fail 'analyze failed' }
    & $ReportExe --input (Join-Path $outDir 'analysis.json') --output (Join-Path $outDir 'report.html')
    if ($LASTEXITCODE -ne 0) { Fail 'report failed' }
    @"
========================================================
report done:
  analysis: $(Join-Path $outDir 'analysis.json')
  report:   $(Join-Path $outDir 'report.html')
========================================================
"@
}

function Invoke-CmdStop {
    if (-not (Load-State)) { Log 'no state file; stopping by conf/output pattern' }
    Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @('agent-cassette.exe', 'actrail-kv-receiver.exe') } |
        ForEach-Object {
            $cmd = $_.CommandLine
            $match = ($cmd -like ('*agent-cassette*' + $CassetteConf + '*start*')) -or
                     ($cmd -like ('*actrail-kv-receiver*' + $ReceiverOut + '*'))
            if ($match) {
                Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
                Log ('stopped pid ' + $_.ProcessId)
            }
        }
    Pass 'stop finished'
}

$command = $args[0]
if (-not $command -or $command -in @('-h', '--help', 'help')) { Show-Usage }
# Accept both "--init" and "init" forms, same as fullchain-deploy-linux.sh.
if ($command.StartsWith('--')) { $command = $command.Substring(1) }
switch ($command) {
    'init'   { Invoke-CmdInit }
    'env'    { Invoke-CmdEnv }
    'health' { Invoke-CmdHealth }
    'report' { Invoke-CmdReport }
    'stop'   { Invoke-CmdStop }
    default  { Show-Usage }
}
