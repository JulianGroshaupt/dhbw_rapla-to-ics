# FROM rust:alpine AS build
FROM ekidd/rust-musl-builder:stable AS build

WORKDIR /app
RUN sudo chown -R rust:rust .

# RUN apk update
# RUN apk add build-base
# RUN apk add openssl libressl-dev

COPY . .

RUN cargo build --release


FROM alpine

WORKDIR /app

RUN apk update
RUN apk add openssl libressl-dev

# RUN apt update
# RUN apt-get install -y pkg-config openssl libssl-dev
# RUN apt-get install -y ca-certificates openssl libssl-dev

COPY --from=build /app/target/x86_64-unknown-linux-musl/release/dhbw_rapla-to-ics /app
# COPY --from=build /app/target/release/dhbw_rapla-to-ics /app

CMD [ "./dhbw_rapla-to-ics" ]