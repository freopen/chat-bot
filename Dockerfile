FROM rust:alpine as builder
RUN apk add --no-cache musl-dev protoc
WORKDIR /usr/src/freopen_chat_bot
COPY . .
RUN cargo install --path .

FROM alpine
COPY assets ./assets
COPY --from=builder /usr/local/cargo/bin/freopen_chat_bot /usr/local/bin/freopen_chat_bot
CMD ["freopen_chat_bot"]