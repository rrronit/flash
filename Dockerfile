FROM rust:1.67 as builder


# Install dependencies for Rust, Isolate, and compilers/interpreters
RUN apt-get update && \
    apt-get install -y curl gcc g++ python3 openjdk-17-jdk nodejs

RUN apt-get update && apt-get install -y --no-install-recommends cron libpq-dev sudo



RUN set -xe && \
    apt-get update && \
    apt-get install -y --no-install-recommends locales && \
    rm -rf /var/lib/apt/lists/* && \
    echo "en_US.UTF-8 UTF-8" > /etc/locale.gen && \
    locale-gen
ENV LANG=en_US.UTF-8 LANGUAGE=en_US:en LC_ALL=en_US.UTF-8

RUN set -xe && \
    apt-get update && \
    apt-get install -y --no-install-recommends git libcap-dev && \
    rm -rf /var/lib/apt/lists/* && \
    git clone https://github.com/ioi/isolate.git /tmp/isolate && \
    cd /tmp/isolate && \
    make -j$(nproc) install && \
    rm -rf /tmp/*



RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      cron \
      libpq-dev \
      sudo 

EXPOSE 3000

WORKDIR /api/
# RUN apt-get update && apt-get install -y pkg-config libcap-dev 
# RUN apt-get update && apt-get install -y libsystemd-dev asciidoc build-essential  
# RUN apt-get update && apt-get install -y make git libcap-dev sudo  libpq-dev


# RUN mkdir -p /api/tmp/ && \
#     chown nobody: /api/tmp/ && \
#     chmod 777 /api/tmp/

COPY tmp tmp

RUN useradd -u 1000 -m -r flash && \
    echo "flash ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers && \
    chown flash: /api/tmp/

USER flash

CMD ["sleep", "infinity"]

