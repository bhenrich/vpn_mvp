package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"vpnmvp/auth/internal/db"
	"vpnmvp/auth/internal/handlers"
)

func main() {
	addr := env("ADDR", ":8080")
	dsn := mustEnv("POSTGRES_DSN")

	pool, err := db.Connect(context.Background(), dsn)
	if err != nil {
		log.Fatalf("db connect: %v", err)
	}
	defer pool.Close()

	if err := db.Migrate(context.Background(), pool); err != nil {
		log.Fatalf("db migrate: %v", err)
	}
	if os.Getenv("SEED_TEST_USER") == "true" {
		email := env("TEST_USER_EMAIL", "test")
		pass := env("TEST_USER_PASSWORD", "test")
		if err := db.EnsureUser(context.Background(), pool, email, pass); err != nil {
			log.Printf("seed user failed: %v", err)
		} else {
			log.Printf("seed user ensured: %s", email)
		}
	}

	h := handlers.New(pool)
	mux := http.NewServeMux()

	mux.HandleFunc("POST /api/v1/auth/signup", withJSON(h.Signup))
	mux.HandleFunc("POST /api/v1/auth/login", withJSON(h.Login))
	mux.HandleFunc("POST /api/v1/auth/refresh", withJSON(h.Refresh))
	mux.HandleFunc("GET /api/v1/vpn/profile", h.Profile)
	mux.HandleFunc("POST /api/v1/vpn/verify", withJSON(h.Verify)) // internal use

	srv := &http.Server{
		Addr:              addr,
		Handler:           logRequests(mux),
		ReadHeaderTimeout: 10 * time.Second,
	}

	go func() {
		log.Printf("auth server listening on %s", addr)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("listen: %v", err)
		}
	}()

	// Graceful shutdown
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)
	<-stop
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	_ = srv.Shutdown(ctx)
}

func env(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

func mustEnv(key string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	log.Fatalf("missing required env %s", key)
	return ""
}

func withJSON(h func(http.ResponseWriter, *http.Request)) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		h(w, r)
	}
}

func logRequests(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		log.Printf("%s %s %s", r.Method, r.URL.Path, time.Since(start))
	})
}


