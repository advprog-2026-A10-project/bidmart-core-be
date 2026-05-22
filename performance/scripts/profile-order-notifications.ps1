param(
    [string]$BaseUrl = "http://127.0.0.1:8081/api/v1",
    [int]$Iterations = 50,
    [int]$Warmup = 5,
    [int]$ApdexSatisfiedMs = 500,
    [string]$BearerToken = "",
    [string]$Cookie = "",
    [string]$UserId = "",
    [string]$OrderId = "",
    [string]$NotificationId = "",
    [string]$OutputDir = ".\performance\results"
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Net.Http

function Join-Url {
    param(
        [string]$Base,
        [string]$Path
    )

    return $Base.TrimEnd("/") + "/" + $Path.TrimStart("/")
}

function Add-Query {
    param(
        [string]$Path,
        [hashtable]$Params
    )

    $items = @()
    foreach ($key in $Params.Keys) {
        $value = $Params[$key]
        if ($null -ne $value -and "$value" -ne "") {
            $items += ("{0}={1}" -f [uri]::EscapeDataString($key), [uri]::EscapeDataString("$value"))
        }
    }

    if ($items.Count -eq 0) {
        return $Path
    }

    return "${Path}?" + ($items -join "&")
}

function Get-Percentile {
    param(
        [double[]]$Values,
        [double]$Percentile
    )

    if ($Values.Count -eq 0) {
        return 0
    }

    $sorted = $Values | Sort-Object
    $index = [math]::Ceiling(($Percentile / 100.0) * $sorted.Count) - 1
    $index = [math]::Max(0, [math]::Min($index, $sorted.Count - 1))
    return [math]::Round([double]$sorted[$index], 2)
}

function Invoke-TimedRequest {
    param(
        [System.Net.Http.HttpClient]$Client,
        [string]$Method,
        [string]$Url
    )

    $request = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::new($Method), $Url)
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

    try {
        $response = $Client.SendAsync($request).GetAwaiter().GetResult()
        $body = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        $stopwatch.Stop()

        return [pscustomobject]@{
            StatusCode = [int]$response.StatusCode
            ElapsedMs = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 2)
            BodyBytes = [System.Text.Encoding]::UTF8.GetByteCount($body)
            Error = ""
        }
    }
    catch {
        $stopwatch.Stop()
        return [pscustomobject]@{
            StatusCode = 0
            ElapsedMs = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 2)
            BodyBytes = 0
            Error = $_.Exception.Message
        }
    }
    finally {
        $request.Dispose()
        if ($null -ne $response) {
            $response.Dispose()
        }
    }
}

if ($Iterations -lt 1) {
    throw "Iterations must be at least 1."
}

if ($Warmup -lt 0) {
    throw "Warmup must not be negative."
}

if ($ApdexSatisfiedMs -lt 1) {
    throw "ApdexSatisfiedMs must be at least 1."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$handler = [System.Net.Http.HttpClientHandler]::new()
$handler.UseCookies = $false
$client = [System.Net.Http.HttpClient]::new($handler)
$client.Timeout = [TimeSpan]::FromSeconds(30)

if ($BearerToken.Trim() -ne "") {
    $client.DefaultRequestHeaders.Authorization =
        [System.Net.Http.Headers.AuthenticationHeaderValue]::new("Bearer", $BearerToken.Trim())
}

if ($Cookie.Trim() -ne "") {
    $client.DefaultRequestHeaders.TryAddWithoutValidation("Cookie", $Cookie.Trim()) | Out-Null
}

$client.DefaultRequestHeaders.Accept.ParseAdd("application/json")

$commonParams = @{}
if ($UserId.Trim() -ne "") {
    $commonParams["userId"] = $UserId.Trim()
}

$targets = @(
    [pscustomobject]@{ Name = "buyer_orders_all"; Method = "GET"; Path = Add-Query "/orders" $commonParams },
    [pscustomobject]@{ Name = "buyer_orders_active"; Method = "GET"; Path = Add-Query "/orders" ($commonParams + @{ stage = "active" }) },
    [pscustomobject]@{ Name = "buyer_orders_processing"; Method = "GET"; Path = Add-Query "/orders" ($commonParams + @{ stage = "processing" }) },
    [pscustomobject]@{ Name = "seller_orders_all"; Method = "GET"; Path = Add-Query "/seller/orders" $commonParams },
    [pscustomobject]@{ Name = "notifications_all"; Method = "GET"; Path = Add-Query "/notifications" ($commonParams + @{ limit = "20" }) },
    [pscustomobject]@{ Name = "notifications_unread"; Method = "GET"; Path = Add-Query "/notifications" ($commonParams + @{ limit = "20"; unreadOnly = "true" }) }
)

if ($OrderId.Trim() -ne "") {
    $targets += [pscustomobject]@{ Name = "buyer_order_detail"; Method = "GET"; Path = "/orders/$($OrderId.Trim())" }
    $targets += [pscustomobject]@{ Name = "seller_order_detail"; Method = "GET"; Path = "/seller/orders/$($OrderId.Trim())" }
}

if ($NotificationId.Trim() -ne "") {
    $targets += [pscustomobject]@{ Name = "notification_detail"; Method = "GET"; Path = "/notifications/$($NotificationId.Trim())" }
}

Write-Host "Profiling $($targets.Count) read-only endpoint(s) at $BaseUrl"
Write-Host "Warmup=$Warmup Iterations=$Iterations APDEX_T=$ApdexSatisfiedMs ms"

foreach ($target in $targets) {
    $url = Join-Url $BaseUrl $target.Path
    for ($i = 0; $i -lt $Warmup; $i++) {
        Invoke-TimedRequest -Client $client -Method $target.Method -Url $url | Out-Null
    }
}

$rows = New-Object System.Collections.Generic.List[object]
$startedAt = Get-Date

foreach ($target in $targets) {
    $url = Join-Url $BaseUrl $target.Path
    Write-Host ("{0} {1}" -f $target.Method, $target.Path)

    for ($i = 1; $i -le $Iterations; $i++) {
        $result = Invoke-TimedRequest -Client $client -Method $target.Method -Url $url
        $rows.Add([pscustomobject]@{
            Timestamp = (Get-Date).ToString("o")
            Endpoint = $target.Name
            Method = $target.Method
            Path = $target.Path
            Iteration = $i
            StatusCode = $result.StatusCode
            ElapsedMs = $result.ElapsedMs
            BodyBytes = $result.BodyBytes
            Error = $result.Error
        })
    }
}

$summary = foreach ($group in ($rows | Group-Object Endpoint)) {
    $latencies = @($group.Group | ForEach-Object { [double]$_.ElapsedMs })
    $successRows = @($group.Group | Where-Object { $_.StatusCode -ge 200 -and $_.StatusCode -lt 400 })
    $satisfied = @($group.Group | Where-Object { $_.ElapsedMs -le $ApdexSatisfiedMs }).Count
    $tolerated = @($group.Group | Where-Object {
        $_.ElapsedMs -gt $ApdexSatisfiedMs -and $_.ElapsedMs -le (4 * $ApdexSatisfiedMs)
    }).Count
    $apdex = [math]::Round(($satisfied + ($tolerated / 2.0)) / $group.Count, 3)

    [pscustomobject]@{
        Endpoint = $group.Name
        Count = $group.Count
        SuccessCount = $successRows.Count
        ErrorCount = $group.Count - $successRows.Count
        AverageMs = [math]::Round(($latencies | Measure-Object -Average).Average, 2)
        P50Ms = Get-Percentile -Values $latencies -Percentile 50
        P95Ms = Get-Percentile -Values $latencies -Percentile 95
        P99Ms = Get-Percentile -Values $latencies -Percentile 99
        MaxMs = [math]::Round(($latencies | Measure-Object -Maximum).Maximum, 2)
        Apdex = $apdex
    }
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$csvPath = Join-Path $OutputDir "order-notification-profile-$timestamp.csv"
$jsonPath = Join-Path $OutputDir "order-notification-profile-$timestamp.json"

$rows | Export-Csv -NoTypeInformation -Path $csvPath

[pscustomobject]@{
    StartedAt = $startedAt.ToString("o")
    FinishedAt = (Get-Date).ToString("o")
    BaseUrl = $BaseUrl
    Iterations = $Iterations
    Warmup = $Warmup
    ApdexSatisfiedMs = $ApdexSatisfiedMs
    Targets = $targets
    Summary = $summary
} | ConvertTo-Json -Depth 6 | Set-Content -Path $jsonPath -Encoding UTF8

$summary | Format-Table -AutoSize
Write-Host "CSV:  $csvPath"
Write-Host "JSON: $jsonPath"

$client.Dispose()
$handler.Dispose()
