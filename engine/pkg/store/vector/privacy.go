package vector

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
)

// PrivacyManager provides GDPR-compliant deletion and pseudonymization (BG-B5)
type PrivacyManager struct {
	store ScopedVectorStore
}

func NewPrivacyManager(store ScopedVectorStore) *PrivacyManager {
	return &PrivacyManager{store: store}
}

// PseudonymizeIdentifier hashes sensitive identifiers (e.g. user/crew IDs)
func PseudonymizeIdentifier(raw string) string {
	h := sha256.Sum256([]byte(raw))
	return "pseudo-" + hex.EncodeToString(h[:8])
}

// DeleteByScope deletes all documents matching the target scope (GDPR right to be forgotten)
func (p *PrivacyManager) DeleteByScope(ctx context.Context, scope map[string]string, docIDs []string) error {
	for _, id := range docIDs {
		if err := p.store.DeleteScoped(ctx, id, scope); err != nil {
			return err
		}
	}
	return nil
}
