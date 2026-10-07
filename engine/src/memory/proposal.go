package memory

import (
	"context"
	"fmt"
	"sort"
	"sync"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
)

// ProposalStatus is the consent lifecycle of a learned correction (F3-3).
// Learning-by-consent (docs/finalize/01, fundamentals): a correction becomes a
// rule only AFTER the Pirate King approves it — never auto-written.
type ProposalStatus string

const (
	ProposalPending  ProposalStatus = "pending"
	ProposalApproved ProposalStatus = "approved"
	ProposalRejected ProposalStatus = "rejected"
	ProposalReverted ProposalStatus = "reverted"
)

// Feedback captures user/Owner comments or corrections on a proposal (BG-F4).
type Feedback struct {
	ID        string    `json:"id"`
	Author    string    `json:"author"`
	Comment   string    `json:"comment"`
	CreatedAt time.Time `json:"created_at"`
}

// MemoryProposal is a proposed learning (a rule) awaiting consent. It is scoped
// (Fleet/Workspace/Project/Ship/Crew/Run), versioned, and reversible — a correction
// is never silently self-applied.
type MemoryProposal struct {
	ID          string            `json:"id"`
	Rule        string            `json:"rule"`             // the correction/preference text
	Scope       map[string]string `json:"scope"`            // where it applies (reused by Document.Scope)
	Status      ProposalStatus    `json:"status"`
	Version     int               `json:"version"`          // monotonic per (ID lineage); supports supersede
	Reversible  bool              `json:"reversible"`       // approved writes can be reverted
	DocumentID  string            `json:"document_id,omitempty"` // set once approved+written
	Feedbacks   []Feedback        `json:"feedbacks,omitempty"`
	RuleHistory map[int]string    `json:"rule_history,omitempty"` // version -> rule content history
	CreatedAt   time.Time         `json:"created_at"`
	DecidedAt   *time.Time        `json:"decided_at,omitempty"`
}

// ProposalStore records memory proposals and gates their write behind approval.
type ProposalStore interface {
	// Propose records a correction as PENDING. It writes nothing to the vector store.
	Propose(ctx context.Context, rule string, scope map[string]string) (*MemoryProposal, error)
	// Approve writes the rule to the vector store (scoped) and marks it approved.
	// Only a pending proposal can be approved.
	Approve(ctx context.Context, id string) (*MemoryProposal, error)
	// Reject marks a pending proposal rejected; nothing is written.
	Reject(ctx context.Context, id string) (*MemoryProposal, error)
	// Revert undoes an approved proposal: removes the written document (reversible).
	Revert(ctx context.Context, id string) (*MemoryProposal, error)
	// AddFeedback records feedback on a proposal (BG-F4).
	AddFeedback(ctx context.Context, id string, author string, comment string) (*MemoryProposal, error)
	// RollbackToVersion rolls back an approved proposal to a prior version (BG-F4).
	RollbackToVersion(ctx context.Context, id string, targetVersion int) (*MemoryProposal, error)
	// Get returns one proposal by id.
	Get(ctx context.Context, id string) (*MemoryProposal, error)
	// List returns proposals, optionally filtered by status ("" = all), newest first.
	List(ctx context.Context, status ProposalStatus) ([]*MemoryProposal, error)
	// SyncFromGateway loads persistent learned rules from Rust SystemGateway (BL-01).
	SyncFromGateway(ctx context.Context) error
}

// revertableStore is the subset of VectorStore the proposal store needs. A store
// that also supports Delete is reversible; one that does not degrades to
// non-reversible approval (the proposal is still recorded, just not revertable).
type revertableStore interface {
	Delete(ctx context.Context, id string) error
}

type proposalStore struct {
	mu        sync.Mutex
	vectors   VectorStore
	gateway   client.SystemGatewayClient
	proposals map[string]*MemoryProposal
	seq       int
}

// NewProposalStore builds a consent-gated learning store over a VectorStore.
func NewProposalStore(vectors VectorStore) ProposalStore {
	return NewProposalStoreWithGateway(vectors, nil)
}

// NewProposalStoreWithGateway builds a consent-gated learning store backed by
// a persistent Rust SystemGateway client (BL-01).
func NewProposalStoreWithGateway(vectors VectorStore, gateway client.SystemGatewayClient) ProposalStore {
	return &proposalStore{
		vectors:   vectors,
		gateway:   gateway,
		proposals: make(map[string]*MemoryProposal),
	}
}

func (p *proposalStore) Propose(ctx context.Context, rule string, scope map[string]string) (*MemoryProposal, error) {
	if rule == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "proposal rule cannot be empty", appErrors.LayerService)
	}
	p.mu.Lock()
	defer p.mu.Unlock()

	p.seq++
	// Version supersedes any earlier approved proposal with the identical rule+scope.
	version := 1
	var prevHistory map[int]string
	for _, ex := range p.proposals {
		if scopeEqual(ex.Scope, scope) && ex.Version >= version {
			version = ex.Version + 1
			if ex.RuleHistory != nil {
				prevHistory = ex.RuleHistory
			}
		}
	}

	history := make(map[int]string)
	if prevHistory != nil {
		for k, v := range prevHistory {
			history[k] = v
		}
	}
	history[version] = rule

	prop := &MemoryProposal{
		ID:          fmt.Sprintf("mp-%d", p.seq),
		Rule:        rule,
		Scope:       cloneScope(scope),
		Status:      ProposalPending,
		Version:     version,
		Reversible:  true,
		RuleHistory: history,
		CreatedAt:   time.Now(),
	}
	p.proposals[prop.ID] = prop
	return clonePro(prop), nil
}

func (p *proposalStore) Approve(ctx context.Context, id string) (*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()

	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}
	if prop.Status != ProposalPending {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, fmt.Sprintf("only a pending proposal can be approved; current status %q", prop.Status), appErrors.LayerService)
	}

	// Write the rule as a scoped document. Embedding is a single-dim placeholder:
	// the real embedding is computed by the retrieval pipeline on index; what matters
	// for the consent boundary is that the write happens ONLY on approval, scoped.
	docID := "mem-" + prop.ID
	doc := &Document{
		ID:        docID,
		Content:   prop.Rule,
		Embedding: []float32{1},
		Metadata:  map[string]string{"kind": "learned_rule", "proposal_id": prop.ID},
		Scope:     cloneScope(prop.Scope),
	}
	if err := p.vectors.Store(ctx, doc); err != nil {
		return nil, fmt.Errorf("approve: write learned rule: %w", err)
	}

	// BL-01: Persist to Rust SystemGateway via gRPC bridge
	if p.gateway != nil {
		if resp, err := p.gateway.StoreLearnedRule(ctx, prop.Rule, prop.Scope, prop.Reversible); err == nil && resp != nil && resp.Id != "" {
			docID = resp.Id
		}
	}

	now := time.Now()
	prop.Status = ProposalApproved
	prop.DocumentID = docID
	prop.DecidedAt = &now
	// Reversible only if the backing store can delete the written document.
	if _, ok := p.vectors.(revertableStore); !ok {
		prop.Reversible = false
	}
	return clonePro(prop), nil
}

func (p *proposalStore) Reject(ctx context.Context, id string) (*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()

	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}
	if prop.Status != ProposalPending {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, fmt.Sprintf("only a pending proposal can be rejected; current status %q", prop.Status), appErrors.LayerService)
	}
	now := time.Now()
	prop.Status = ProposalRejected
	prop.DecidedAt = &now
	return clonePro(prop), nil
}

func (p *proposalStore) Revert(ctx context.Context, id string) (*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()

	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}
	if prop.Status != ProposalApproved {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, fmt.Sprintf("only an approved proposal can be reverted; current status %q", prop.Status), appErrors.LayerService)
	}
	rev, ok := p.vectors.(revertableStore)
	if !ok || !prop.Reversible {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "backing store does not support revert", appErrors.LayerService)
	}
	if err := rev.Delete(ctx, prop.DocumentID); err != nil {
		return nil, fmt.Errorf("revert: delete learned rule: %w", err)
	}
	now := time.Now()
	prop.Status = ProposalReverted
	prop.DecidedAt = &now
	return clonePro(prop), nil
}

func (p *proposalStore) AddFeedback(ctx context.Context, id string, author string, comment string) (*MemoryProposal, error) {
	if author == "" || comment == "" {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "author and comment cannot be empty", appErrors.LayerService)
	}
	p.mu.Lock()
	defer p.mu.Unlock()

	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}

	fb := Feedback{
		ID:        fmt.Sprintf("fb-%s-%d", prop.ID, len(prop.Feedbacks)+1),
		Author:    author,
		Comment:   comment,
		CreatedAt: time.Now(),
	}
	prop.Feedbacks = append(prop.Feedbacks, fb)
	return clonePro(prop), nil
}

func (p *proposalStore) RollbackToVersion(ctx context.Context, id string, targetVersion int) (*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()

	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}
	if prop.Status != ProposalApproved {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "only an approved proposal can be rolled back", appErrors.LayerService)
	}
	if targetVersion <= 0 || targetVersion >= prop.Version {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, fmt.Sprintf("invalid target version %d for current version %d", targetVersion, prop.Version), appErrors.LayerService)
	}
	targetRule, exists := prop.RuleHistory[targetVersion]
	if !exists {
		return nil, appErrors.New(appErrors.CodeNotFound, fmt.Sprintf("rule version %d not found in history", targetVersion), appErrors.LayerService)
	}

	rev, ok := p.vectors.(revertableStore)
	if !ok || !prop.Reversible {
		return nil, appErrors.New(appErrors.CodeInvalidArgument, "backing store does not support revert/rollback", appErrors.LayerService)
	}

	if prop.DocumentID != "" {
		_ = rev.Delete(ctx, prop.DocumentID)
	}

	docID := fmt.Sprintf("mem-%s-v%d", prop.ID, targetVersion)
	doc := &Document{
		ID:        docID,
		Content:   targetRule,
		Embedding: []float32{1},
		Metadata:  map[string]string{"kind": "learned_rule", "proposal_id": prop.ID, "rolled_back_from": fmt.Sprintf("v%d", prop.Version)},
		Scope:     cloneScope(prop.Scope),
	}
	if err := p.vectors.Store(ctx, doc); err != nil {
		return nil, fmt.Errorf("rollback: write rolled back rule: %w", err)
	}

	now := time.Now()
	prop.Rule = targetRule
	prop.Version = targetVersion
	prop.DocumentID = docID
	prop.DecidedAt = &now

	return clonePro(prop), nil
}

func (p *proposalStore) Get(ctx context.Context, id string) (*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()
	prop, ok := p.proposals[id]
	if !ok {
		return nil, appErrors.New(appErrors.CodeNotFound, "proposal not found", appErrors.LayerService)
	}
	return clonePro(prop), nil
}

func (p *proposalStore) List(ctx context.Context, status ProposalStatus) ([]*MemoryProposal, error) {
	p.mu.Lock()
	defer p.mu.Unlock()
	out := make([]*MemoryProposal, 0, len(p.proposals))
	for _, prop := range p.proposals {
		if status == "" || prop.Status == status {
			out = append(out, clonePro(prop))
		}
	}
	// Newest first (seq is encoded in CreatedAt order; sort by CreatedAt desc then ID).
	sort.Slice(out, func(i, j int) bool {
		if !out[i].CreatedAt.Equal(out[j].CreatedAt) {
			return out[i].CreatedAt.After(out[j].CreatedAt)
		}
		return out[i].ID > out[j].ID
	})
	return out, nil
}

func (p *proposalStore) SyncFromGateway(ctx context.Context) error {
	if p.gateway == nil {
		return nil
	}

	rules, err := p.gateway.RetrieveLearnedRules(ctx, "", "")
	if err != nil {
		return fmt.Errorf("sync from gateway: %w", err)
	}

	p.mu.Lock()
	defer p.mu.Unlock()

	for _, r := range rules {
		prop := &MemoryProposal{
			ID:         r.Id,
			Rule:       r.Rule,
			Scope:      cloneScope(r.Scope),
			Status:     ProposalApproved,
			Version:    1,
			Reversible: r.Reversible,
			DocumentID: r.Id,
			CreatedAt:  time.Now(),
		}
		if r.Reverted {
			prop.Status = ProposalReverted
		}
		p.proposals[prop.ID] = prop
	}
	return nil
}

func cloneScope(s map[string]string) map[string]string {
	if s == nil {
		return nil
	}
	out := make(map[string]string, len(s))
	for k, v := range s {
		out[k] = v
	}
	return out
}

func scopeEqual(a, b map[string]string) bool {
	if len(a) != len(b) {
		return false
	}
	for k, v := range a {
		if b[k] != v {
			return false
		}
	}
	return true
}

func clonePro(p *MemoryProposal) *MemoryProposal {
	cp := *p
	cp.Scope = cloneScope(p.Scope)
	if p.Feedbacks != nil {
		cp.Feedbacks = make([]Feedback, len(p.Feedbacks))
		copy(cp.Feedbacks, p.Feedbacks)
	}
	if p.RuleHistory != nil {
		cp.RuleHistory = make(map[int]string, len(p.RuleHistory))
		for k, v := range p.RuleHistory {
			cp.RuleHistory[k] = v
		}
	}
	if p.DecidedAt != nil {
		t := *p.DecidedAt
		cp.DecidedAt = &t
	}
	return &cp
}
