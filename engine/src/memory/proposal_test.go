package memory

import (
	"context"
	"testing"
)

// F3-3 learning-by-consent lifecycle: a correction is written to memory ONLY
// after approval, scoped and reversible.
func TestProposalStore_Lifecycle(t *testing.T) {
	ctx := context.Background()
	vs := NewVectorStore()
	ps := NewProposalStore(vs)

	scope := map[string]string{"ship": "dev-ship", "project": "galleon"}

	// Propose: pending, nothing written yet.
	prop, err := ps.Propose(ctx, "Always run cargo check -p before claiming done", scope)
	if err != nil {
		t.Fatalf("Propose: %v", err)
	}
	if prop.Status != ProposalPending {
		t.Fatalf("expected pending, got %q", prop.Status)
	}
	if res, _ := vs.SearchByText(ctx, "cargo check", 5); len(res) != 0 {
		t.Fatalf("nothing must be written before approval, found %d docs", len(res))
	}

	// Approve: now written, scoped, searchable.
	approved, err := ps.Approve(ctx, prop.ID)
	if err != nil {
		t.Fatalf("Approve: %v", err)
	}
	if approved.Status != ProposalApproved || approved.DocumentID == "" {
		t.Fatalf("expected approved+written, got %+v", approved)
	}
	scoped, _ := vs.SearchWithScope(ctx, []float32{1}, 5, scope)
	if len(scoped) != 1 {
		t.Fatalf("approved rule must be written in-scope, found %d", len(scoped))
	}
	// Out-of-scope search must NOT see it.
	other, _ := vs.SearchWithScope(ctx, []float32{1}, 5, map[string]string{"ship": "other-ship"})
	if len(other) != 0 {
		t.Fatalf("rule leaked outside its scope, found %d", len(other))
	}

	// Approving again is refused (not pending).
	if _, err := ps.Approve(ctx, prop.ID); err == nil {
		t.Fatal("re-approving an approved proposal must fail")
	}

	// Revert: reversible — the written rule is removed.
	reverted, err := ps.Revert(ctx, prop.ID)
	if err != nil {
		t.Fatalf("Revert: %v", err)
	}
	if reverted.Status != ProposalReverted {
		t.Fatalf("expected reverted, got %q", reverted.Status)
	}
	if res, _ := vs.SearchWithScope(ctx, []float32{1}, 5, scope); len(res) != 0 {
		t.Fatalf("reverted rule must be gone, found %d", len(res))
	}
}

// A rejected proposal never writes anything.
func TestProposalStore_RejectWritesNothing(t *testing.T) {
	ctx := context.Background()
	vs := NewVectorStore()
	ps := NewProposalStore(vs)

	prop, err := ps.Propose(ctx, "Never push to master", nil)
	if err != nil {
		t.Fatalf("Propose: %v", err)
	}
	if _, err := ps.Reject(ctx, prop.ID); err != nil {
		t.Fatalf("Reject: %v", err)
	}
	if res, _ := vs.SearchByText(ctx, "master", 5); len(res) != 0 {
		t.Fatalf("rejected proposal must write nothing, found %d", len(res))
	}
	// Approving a rejected proposal is refused.
	if _, err := ps.Approve(ctx, prop.ID); err == nil {
		t.Fatal("approving a rejected proposal must fail")
	}
}

// Re-proposing an identical rule+scope bumps the version (supersede, versioned).
func TestProposalStore_VersionBumpsOnSupersede(t *testing.T) {
	ctx := context.Background()
	ps := NewProposalStore(NewVectorStore())
	scope := map[string]string{"project": "galleon"}

	first, _ := ps.Propose(ctx, "prefer targeted builds", scope)
	second, _ := ps.Propose(ctx, "prefer targeted builds", scope)
	if second.Version <= first.Version {
		t.Fatalf("expected version bump, got v%d then v%d", first.Version, second.Version)
	}
}

// BG-F4: Feedback on proposals.
func TestProposalStore_Feedback(t *testing.T) {
	ctx := context.Background()
	ps := NewProposalStore(NewVectorStore())

	prop, err := ps.Propose(ctx, "Check cargo fmt first", nil)
	if err != nil {
		t.Fatalf("Propose: %v", err)
	}

	updated, err := ps.AddFeedback(ctx, prop.ID, "PirateKing", "Please also check clippy")
	if err != nil {
		t.Fatalf("AddFeedback: %v", err)
	}
	if len(updated.Feedbacks) != 1 {
		t.Fatalf("expected 1 feedback, got %d", len(updated.Feedbacks))
	}
	if updated.Feedbacks[0].Author != "PirateKing" || updated.Feedbacks[0].Comment != "Please also check clippy" {
		t.Fatalf("unexpected feedback data: %+v", updated.Feedbacks[0])
	}
}

// BG-F4: Rollback to previous version.
func TestProposalStore_RollbackToVersion(t *testing.T) {
	ctx := context.Background()
	vs := NewVectorStore()
	ps := NewProposalStore(vs)
	scope := map[string]string{"project": "galleon"}

	// v1
	v1, err := ps.Propose(ctx, "Rule V1: do x", scope)
	if err != nil {
		t.Fatalf("Propose v1: %v", err)
	}
	_, err = ps.Approve(ctx, v1.ID)
	if err != nil {
		t.Fatalf("Approve v1: %v", err)
	}

	// v2 (supersedes v1)
	v2, err := ps.Propose(ctx, "Rule V2: do y", scope)
	if err != nil {
		t.Fatalf("Propose v2: %v", err)
	}
	approvedV2, err := ps.Approve(ctx, v2.ID)
	if err != nil {
		t.Fatalf("Approve v2: %v", err)
	}
	if approvedV2.Version != 2 {
		t.Fatalf("expected version 2, got %d", approvedV2.Version)
	}

	// Rollback v2 to v1
	rolledBack, err := ps.RollbackToVersion(ctx, v2.ID, 1)
	if err != nil {
		t.Fatalf("RollbackToVersion: %v", err)
	}
	if rolledBack.Version != 1 || rolledBack.Rule != "Rule V1: do x" {
		t.Fatalf("expected rollback to v1 with 'Rule V1: do x', got v%d rule %q", rolledBack.Version, rolledBack.Rule)
	}

	// Verify vector store contains the rolled back rule
	docs, err := vs.SearchByText(ctx, "Rule V1", 5)
	if err != nil || len(docs) == 0 {
		t.Fatalf("expected to find rolled back rule in vector store, err: %v, docs: %d", err, len(docs))
	}
}

