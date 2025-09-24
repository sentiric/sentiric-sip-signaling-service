#!/bin/bash
set -e
set -o pipefail

# Bu script, Docker Compose için son .env dosyasını oluşturur.
# Sadece temel Bash özelliklerini kullanarak değişkenleri güvenilir bir şekilde çözer.

PROFILE=${1:-dev}

# --- Temel Dizinleri Tanımla ---
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
INFRA_DIR=$(dirname "$SCRIPT_DIR")
WORKSPACE_DIR=$(dirname "$INFRA_DIR")

# --- Kaynak ve Hedef Dosyaları Tanımla ---
CONFIG_DIR="${WORKSPACE_DIR}/sentiric-config/environments"
OUTPUT_FILE="${INFRA_DIR}/.env.generated"
PROFILE_FILE="${CONFIG_DIR}/profiles/${PROFILE}.env"
TEMP_ENV_FILE=$(mktemp)

# Script sonlandığında geçici dosyayı sil
trap 'rm -f "$TEMP_ENV_FILE"' EXIT

if [ ! -f "$PROFILE_FILE" ]; then
    echo "❌ HATA: Profil dosyası bulunamadı: $PROFILE_FILE"
    exit 1
fi

echo "🔧 Yapılandırma dosyası '${OUTPUT_FILE}' oluşturuluyor (Profil: ${PROFILE})..."

# --- AŞAMA 1: Gerekli tüm .env parçalarını tek bir geçici dosyada birleştir ---

# Önce dinamik değişkenleri ekle
echo "# Dynamically added by Orchestrator (Pre-resolution)" > "$TEMP_ENV_FILE"
DETECTED_LAN_IP=$(ip -4 route get 1.1.1.1 | awk '{print $7; exit}')
echo "DETECTED_LAN_IP=${DETECTED_LAN_IP:-127.0.0.1}" >> "$TEMP_ENV_FILE"
echo "TAG=${TAG:-latest}" >> "$TEMP_ENV_FILE"
echo "CONFIG_REPO_PATH=../sentiric-config" >> "$TEMP_ENV_FILE"
echo "CERTIFICATES_REPO_PATH=../sentiric-certificates" >> "$TEMP_ENV_FILE"
echo "ASSETS_REPO_PATH=../sentiric-assets" >> "$TEMP_ENV_FILE"

# Sonra profil dosyasını oku ve 'source' komutlarını işle
while IFS= read -r line || [[ -n "$line" ]]; do
    line=$(echo "$line" | tr -d '\r')
    # Sadece 'source' ile başlayan satırları işle
    if [[ $line == source* ]]; then
        relative_path=$(echo "$line" | cut -d' ' -f2)
        source_file="${CONFIG_DIR}/${relative_path}"
        if [ -f "$source_file" ]; then
            echo -e "\n# Included from: ${relative_path}" >> "$TEMP_ENV_FILE"
            # Yorum olmayan ve boş olmayan satırları ekle
            grep -vE '^\s*(#|$)' "$source_file" | tr -d '\r' >> "$TEMP_ENV_FILE"
        fi
    fi
done < "$PROFILE_FILE"

# --- AŞAMA 2: Birleştirilmiş dosyayı kullanarak tüm değişkenleri çöz ve nihai dosyayı oluştur ---
# envsubst, bir metindeki ${VAR} veya $VAR ifadelerini ortam değişkenleriyle değiştiren standart bir araçtır.
# Bu en güvenilir yöntemdir.
( set -a; source "$TEMP_ENV_FILE"; envsubst < "$TEMP_ENV_FILE" > "$OUTPUT_FILE" )

echo "✅ Yapılandırma başarıyla oluşturuldu."
chmod +x "$INFRA_DIR/scripts/restart-services.sh" 2>/dev/null || true