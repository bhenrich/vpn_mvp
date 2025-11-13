FROM golang:1.22-alpine AS build
WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY . .
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o /out/auth ./cmd/auth

FROM gcr.io/distroless/base-debian12
WORKDIR /app
COPY --from=build /out/auth /app/auth
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/app/auth"]



