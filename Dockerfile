# Adım 1: Bağımlılıkları kurmak için Node.js'in hafif bir versiyonunu kullan.
FROM node:18-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm install

# Adım 2: Sadece gerekli dosyaları içeren minimal bir imaj oluştur.
FROM node:18-alpine
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .

# Bu servis UDP 5060 portunu dışarıya açacak.
EXPOSE 5060/udp

# Konteyner başladığında çalıştırılacak komut.
CMD [ "npm", "start" ]