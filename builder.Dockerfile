FROM rust:latest

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install libvirt-dev libvirt0 -y

ENV CARGO_HOME=/home/ap/.cargo
ENV PATH=$PATH:/home/ap/.cargo/bin
RUN groupadd -g 1000 appgroup && useradd -u 1000 -g appgroup -d /home/ap -s /sbin/nologin -c "Application User" appuser
RUN mkdir -p /home/ap/.cargo/bin && chown -R appuser:appgroup /home/ap
USER appuser

WORKDIR /app

COPY Cargo.toml .
COPY src/ ./src

RUN cargo build