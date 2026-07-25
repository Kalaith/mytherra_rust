# Advertise this desktop's mytherra-server to the public gateway on a heartbeat.
#
# The published client reaches the world through the `local_gateway` reverse
# proxy on webhatchery.au (apps/local_gateway); the gateway only knows where to
# forward once this script has registered the home box's current address. It
# re-registers every interval so the record never goes stale (the gateway drops
# a service to 502 once its registration ages past GATEWAY_TTL).
#
# Prerequisites:
#   - mytherra-server running locally (.\run-server.ps1) on -Port.
#   - Your router forwards -Port to this desktop (ideally firewalled to accept
#     only webhatchery.au). By default the gateway derives the host from THIS
#     request's source IP — i.e. your public IP — so you never hand it out here.
#   - The gateway admin token, via -Token or $env:GATEWAY_ADMIN_TOKEN.
#
# Examples:
#   $env:GATEWAY_ADMIN_TOKEN = '...'; .\register-server.ps1
#   .\register-server.ps1 -Port 8791 -IntervalSeconds 60
#   .\register-server.ps1 -Target 'https://abc123.trycloudflare.com'   # a tunnel
#   .\register-server.ps1 -Once                                        # one shot

param(
    [string]$GatewayUrl = 'https://webhatchery.au/local_gateway/api',
    [string]$Service = 'mytherra',
    [int]$Port = 8791,
    # An explicit reachable base URL (e.g. an https tunnel). Overrides -Port; when
    # empty the gateway builds http://<your-public-ip>:<Port> from the request IP.
    [string]$Target = '',
    [string]$Token = $env:GATEWAY_ADMIN_TOKEN,
    [int]$IntervalSeconds = 60,
    [switch]$Once
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($Token)) {
    Write-Error "No gateway token. Pass -Token or set `$env:GATEWAY_ADMIN_TOKEN."
    exit 1
}

$endpoint = ($GatewayUrl.TrimEnd('/')) + '/register'
$body = if ([string]::IsNullOrWhiteSpace($Target)) {
    @{ service = $Service; port = $Port }
} else {
    @{ service = $Service; target = $Target.TrimEnd('/') }
}
$json = $body | ConvertTo-Json -Compress
$headers = @{ 'X-Gateway-Token' = $Token }

$where = if ($Target) { $Target } else { "http://<your public IP>:$Port" }
Write-Host ""
Write-Host "Gateway   : $endpoint" -ForegroundColor Green
Write-Host "Service   : $Service -> $where" -ForegroundColor Green
Write-Host $(if ($Once) { "Mode      : one shot" } else { "Mode      : heartbeat every ${IntervalSeconds}s (Ctrl+C to stop)" }) -ForegroundColor DarkGray
Write-Host ""

function Send-Registration {
    try {
        $resp = Invoke-RestMethod -Uri $endpoint -Method Post -Headers $headers `
            -ContentType 'application/json' -Body $json -TimeoutSec 15
        $stamp = (Get-Date).ToString('HH:mm:ss')
        Write-Host "[$stamp] registered: $($resp.target) (ttl $($resp.ttl)s)" -ForegroundColor Green
    } catch {
        $stamp = (Get-Date).ToString('HH:mm:ss')
        Write-Host "[$stamp] register failed: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Send-Registration
if ($Once) { return }

while ($true) {
    Start-Sleep -Seconds $IntervalSeconds
    Send-Registration
}
