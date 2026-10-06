package identity

import (
	"encoding/json"
	"net/http"
)

// AuthMiddleware creates an HTTP middleware that extracts and validates caller principal (BG-C3)
func AuthMiddleware(resolver PrincipalResolver) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if resolver == nil {
				next.ServeHTTP(w, r)
				return
			}

			// Attempt to resolve credentials
			principal, err := resolver.Resolve(r.Context(), r)
			if err == nil && principal != nil && principal.IsAuthenticated() {
				ctx := WithPrincipal(r.Context(), principal)
				next.ServeHTTP(w, r.WithContext(ctx))
				return
			}

			// Continue with unmodified context; route-level guards will enforce if required
			next.ServeHTTP(w, r)
		})
	}
}

// RequireAuth enforces that a route must have a verified caller principal
func RequireAuth(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		principal, ok := FromContext(r.Context())
		if !ok || principal == nil || !principal.IsAuthenticated() {
			writeErrorJSON(w, http.StatusUnauthorized, "unauthorized: valid authentication credentials required")
			return
		}
		next.ServeHTTP(w, r)
	})
}

// RequireRole enforces that a caller must hold at least the specified role
func RequireRole(role Role) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			principal, ok := FromContext(r.Context())
			if !ok || principal == nil || !principal.IsAuthenticated() {
				writeErrorJSON(w, http.StatusUnauthorized, "unauthorized: authentication required")
				return
			}
			if !principal.HasRole(role) {
				writeErrorJSON(w, http.StatusForbidden, "forbidden: insufficient permissions for this operation")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

// RequireApprovalAuthority enforces that a caller has authority to approve high-risk actions (S2-SEC-01)
func RequireApprovalAuthority(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		principal, ok := FromContext(r.Context())
		if !ok || principal == nil || !principal.IsAuthenticated() {
			writeErrorJSON(w, http.StatusUnauthorized, "unauthorized: approval requires verified caller principal")
			return
		}
		if !principal.CanApprove() {
			writeErrorJSON(w, http.StatusForbidden, "forbidden: caller principal lacks approval authority")
			return
		}
		next.ServeHTTP(w, r)
	})
}

func writeErrorJSON(w http.ResponseWriter, status int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]any{
		"error":   msg,
		"success": false,
	})
}
