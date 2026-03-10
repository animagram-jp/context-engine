FROM php:8.4-cli

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libssl-dev \
    pkg-config \
    git \
    && rm -rf /var/lib/apt/lists/*

# phpredis
RUN pecl install redis && docker-php-ext-enable redis

# PDO MySQL
RUN docker-php-ext-install pdo_mysql

# Rust toolchain (for building the state-engine extension)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.crateup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# Copy the full crate root to build the extension
COPY . .

RUN cargo build --release

RUN EXT_DIR=$(php-config --extension-dir) && \
    cp target/release/libyour_crate_lib.so $EXT_DIR/your_crate_lib.so && \
    docker-php-ext-enable your_crate_lib

# --- app setup ---
WORKDIR /app

COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

# Copy app sources
COPY examples/app/composer.json examples/app/composer.lock* ./
RUN composer install --no-dev --optimize-autoloader

COPY examples/app/src/      ./src/
COPY examples/app/index.php ./index.php

# Copy Adapters and manifest alongside app
COPY examples/Adapters/ ./Adapters/
COPY examples/manifest/  ./manifest/

EXPOSE 8080
CMD ["php", "-S", "0.0.0.0:8080", "index.php"]
