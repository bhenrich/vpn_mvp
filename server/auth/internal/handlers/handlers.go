package handlers

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
	"github.com/jackc/pgx/v5/pgxpool"
	"golang.org/x/crypto/bcrypt"
)

type Handlers struct {
	db     *pgxpool.Pool
	jwtKey []byte
}

func New(db *pgxpool.Pool) *Handlers {
	key := []byte(os.Getenv("JWT_SIGNING_KEY"))
	if len(key) == 0 {
		key = []byte("dev-insecure-key")
	}
	return &Handlers{db: db, jwtKey: key}
}

type loginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type loginResponse struct {
	AccessToken  string `json:"accessToken"`
	RefreshToken string `json:"refreshToken"`
}

func (h *Handlers) Signup(w http.ResponseWriter, r *http.Request) {
	var req loginRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}
	if req.Email == "" || req.Password == "" {
		http.Error(w, "missing email or password", http.StatusBadRequest)
		return
	}
	hash, _ := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	_, err := h.db.Exec(r.Context(), `INSERT INTO users(email, password_hash) VALUES($1,$2)`, strings.ToLower(req.Email), string(hash))
	if err != nil {
		if pgErr(err, "23505") {
			http.Error(w, "user exists", http.StatusConflict)
			return
		}
		http.Error(w, "db error", http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusCreated)
	io.WriteString(w, `{"ok":true}`)
}

func (h *Handlers) Login(w http.ResponseWriter, r *http.Request) {
	var req loginRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}
	var id int64
	var phash string
	err := h.db.QueryRow(r.Context(), `SELECT id, password_hash FROM users WHERE email=$1`, strings.ToLower(req.Email)).Scan(&id, &phash)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			http.Error(w, "invalid credentials", http.StatusUnauthorized)
			return
		}
		http.Error(w, "db error", http.StatusInternalServerError)
		return
	}
	if bcrypt.CompareHashAndPassword([]byte(phash), []byte(req.Password)) != nil {
		http.Error(w, "invalid credentials", http.StatusUnauthorized)
		return
	}

	access := h.signJWT(req.Email, 15*time.Minute)
	refresh := h.signJWT(req.Email, 7*24*time.Hour)
	json.NewEncoder(w).Encode(loginResponse{
		AccessToken:  access,
		RefreshToken: refresh,
	})
}

type refreshRequest struct {
	RefreshToken string `json:"refreshToken"`
}

func (h *Handlers) Refresh(w http.ResponseWriter, r *http.Request) {
	var req refreshRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "invalid json", http.StatusBadRequest)
		return
	}
	email, err := h.parseJWT(req.RefreshToken)
	if err != nil {
		http.Error(w, "invalid token", http.StatusUnauthorized)
		return
	}
	access := h.signJWT(email, 15*time.Minute)
	refresh := h.signJWT(email, 7*24*time.Hour)
	json.NewEncoder(w).Encode(loginResponse{
		AccessToken:  access,
		RefreshToken: refresh,
	})
}

func (h *Handlers) Profile(w http.ResponseWriter, r *http.Request) {
	email, err := h.authBearer(r.Context(), r.Header.Get("Authorization"))
	if err != nil {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	_ = email // could embed as a comment in profile if desired
	remote := env("OVPN_REMOTE", "127.0.0.1")
	port := env("OVPN_PORT", "1194")
	proto := env("OVPN_PROTO", "udp")

	// Minimal profile (cert-less, relies on user-pass verify). TLS material must match server.
	profile := fmt.Sprintf(`client
dev tun
proto %s
remote %s %s
resolv-retry infinite
nobind
persist-key
persist-tun
verb 3
auth-user-pass

cipher AES-256-GCM
setenv CLIENT_CERT 0
`, proto, remote, port)

	w.Header().Set("Content-Type", "text/plain")
	io.WriteString(w, profile)
}

type verifyRequest struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

// Verify is called by OpenVPN server: username=email, password=accessToken
func (h *Handlers) Verify(w http.ResponseWriter, r *http.Request) {
	var req verifyRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad json", http.StatusBadRequest)
		return
	}
	email, err := h.parseJWT(req.Password)
	if err != nil || !strings.EqualFold(email, req.Username) {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	// Optional: ensure user exists
	var id int64
	if err := h.db.QueryRow(r.Context(), `SELECT id FROM users WHERE email=$1`, strings.ToLower(email)).Scan(&id); err != nil {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	w.WriteHeader(http.StatusOK)
	io.WriteString(w, `{"ok":true}`)
}

func (h *Handlers) signJWT(email string, ttl time.Duration) string {
	claims := jwt.RegisteredClaims{
		Subject:   email,
		ExpiresAt: jwt.NewNumericDate(time.Now().Add(ttl)),
		IssuedAt:  jwt.NewNumericDate(time.Now()),
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	s, _ := token.SignedString(h.jwtKey)
	return s
}

func (h *Handlers) parseJWT(tokenStr string) (string, error) {
	token, err := jwt.ParseWithClaims(tokenStr, &jwt.RegisteredClaims{}, func(token *jwt.Token) (interface{}, error) {
		return h.jwtKey, nil
	})
	if err != nil {
		return "", err
	}
	if claims, ok := token.Claims.(*jwt.RegisteredClaims); ok && token.Valid {
		return claims.Subject, nil
	}
	return "", errors.New("invalid token")
}

func (h *Handlers) authBearer(ctx context.Context, header string) (string, error) {
	if header == "" {
		return "", errors.New("missing")
	}
	parts := strings.SplitN(header, " ", 2)
	if len(parts) != 2 || !strings.EqualFold(parts[0], "Bearer") {
		return "", errors.New("bad header")
	}
	return h.parseJWT(parts[1])
}

func pgErr(err error, code string) bool {
	var pg *pgconn.PgError
	if errors.As(err, &pg) {
		return pg.Code == code
	}
	return false
}

func env(k, d string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return d
}


