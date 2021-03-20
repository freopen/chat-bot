FROM rust:alpine as builder
RUN apk add --no-cache musl-dev openssl-dev perl make
WORKDIR /usr/src/freopen_chat_bot
COPY . .
RUN cargo install --path .

FROM alpine
COPY --from=builder /usr/local/cargo/bin/freopen_chat_bot /usr/local/bin/freopen_chat_bot
COPY assets ./assets
CMD ["freopen_chat_bot"]