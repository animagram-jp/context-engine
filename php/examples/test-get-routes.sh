#!/bin/bash

# GETルートのHTTPステータステスト（curlでdmap-sso-user-idヘッダ付与）する画期的な便利スクリプト
# Usage: ./bin/test-get-routes.sh [user_key]
#
# ローカル向けuser_key (要 artisan db:seed):
#   1  - 開発 (developers, org_id=999999)
#   11 - 医院管理者（三郷診療所, org_id=1）【推奨】
#   16 - 佐藤花子（スタッフ, org_id=1）
#   17 - 田中太郎（スタッフ, org_id=1）

# 【出力例】
# user@PC:~/repo$ make artisan CMD="engine:make --user 11 org 1" && ./scripts/test-get-routes.sh 11
#
# === Testing API Routes (user_key=11) ===
#
# Fetching routes from artisan route:list...
# [302] /
# [401] maintainer/maintainer-users
# [401] maintainer/maintainer-users/create
# [401] maintainer/clinics
# [401] maintainer/clinics/create
# [401] maintainer/dashboard
# ...
# [200] karte/pdf-test-page
# [200] karte/self-paid-groups-data
# [200] karte/set-treatments-data
# [200] karte/shohou-data
# [500] karte/shosaishin-info
# [200] karte/treatment-flow/all
# [200] karte/treatment-groups-data
# [200] karte/users
# ...
# === Summary ===
# ✓ Success (200): 106
# → Redirect (3xx): 5
# ✗ Error (4xx/5xx): 18
# - Skipped (params): 86
# Total tested: 190

SSO_USER_ID=${1:-11}
BASE_URL="https://d-map.test:8000"

# スクリプトのディレクトリを取得
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Testing API Routes (user_key=$SSO_USER_ID) ==="
echo ""

# artisan route:list から GET ルートの URI を抽出
echo "Fetching routes from artisan route:list..."

# artisan route:list を実行して GET ルートを取得
ROUTES=$(cd "$PROJECT_DIR" && docker compose -f deployment/development/docker-compose.yml exec -u 1000:1000 -e HOME=/tmp -T emr-laravel php /var/www/html/artisan route:list --method=GET 2>&1 \
    | grep -E '^\s*(GET|HEAD)' \
    | awk '{print $2}' \
    | grep -v '^$' \
    | grep -v '_debugbar' \
    | grep -v '_ignition' \
    | grep -v 'sanctum/csrf-cookie' \
    | sort -u)

# カウンター
SUCCESS_COUNT=0
ERROR_COUNT=0
REDIRECT_COUNT=0
SKIP_COUNT=0
TOTAL_COUNT=0

# 各ルートをテスト
while IFS= read -r route; do
    # 空行をスキップ
    [ -z "$route" ] && continue

    # {parameter} を含むルートはスキップ（パラメータ必須）
    if [[ "$route" == *"{"* ]]; then
        ((SKIP_COUNT++))
        continue
    fi

    ((TOTAL_COUNT++))
    URL="${BASE_URL}/${route}"

    # HTTP ステータスコードを取得
    STATUS=$(curl -k -s -o /dev/null -w "%{http_code}" \
        -X GET "$URL" \
        -H "request-header-user-key: $SSO_USER_ID" \
        -H "Accept: application/json" \
        -H "X-Requested-With: XMLHttpRequest" \
        --max-time 10 2>/dev/null)

    # ステータスに応じて色分け
    if [[ "$STATUS" == "200" ]]; then
        echo -e "\033[32m[$STATUS]\033[0m $route"
        ((SUCCESS_COUNT++))
    elif [[ "$STATUS" == "302" ]] || [[ "$STATUS" == "301" ]]; then
        echo -e "\033[33m[$STATUS]\033[0m $route"
        ((REDIRECT_COUNT++))
    elif [[ "$STATUS" == "500" ]] || [[ "$STATUS" == "404" ]] || [[ "$STATUS" == "422" ]]; then
        echo -e "\033[31m[$STATUS]\033[0m $route"
        ((ERROR_COUNT++))
    else
        echo -e "[$STATUS] $route"
    fi
done <<< "$ROUTES"

echo ""
echo "=== Summary ==="
echo "✓ Success (200): $SUCCESS_COUNT"
echo "→ Redirect (3xx): $REDIRECT_COUNT"
echo "✗ Error (4xx/5xx): $ERROR_COUNT"
echo "- Skipped (params): $SKIP_COUNT"
echo "Total tested: $TOTAL_COUNT"
