package approval_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/diezy-labs/claw-crew/engine/core/metrics"
	"github.com/diezy-labs/claw-crew/engine/src/approval"
	"github.com/diezy-labs/claw-crew/engine/src/memory"
)

func TestLearningGovernance_Lifecycle(t *testing.T) {
	vs := memory.NewVectorStore()
	ps := memory.NewProposalStore(vs)
	svc := memory.NewLearningGovernanceService(ps)
	handler := approval.NewLearningHandler(svc)

	server := metrics.NewServer(0)
	handler.RegisterHTTP(server)

	// 1. POST /api/v1/learning/proposals (Propose Learning by Consent)
	reqBody := approval.CreateProposalRequest{
		Rule:  "Always enforce fail-closed security boundary",
		Scope: map[string]string{"fleet": "flagship", "ship": "galleon-01"},
	}
	bodyBytes, _ := json.Marshal(reqBody)
	req := httptest.NewRequest(http.MethodPost, "/api/v1/learning/proposals", bytes.NewReader(bodyBytes))
	rec := httptest.NewRecorder()
	server.ServeHTTP(rec, req)

	if rec.Code != http.StatusCreated {
		t.Fatalf("expected 201 Created from POST /api/v1/learning/proposals, got %d: %s", rec.Code, rec.Body.String())
	}

	var createResp map[string]string
	if err := json.Unmarshal(rec.Body.Bytes(), &createResp); err != nil {
		t.Fatalf("failed to decode create response: %v", err)
	}
	propID := createResp["id"]
	if propID == "" {
		t.Fatalf("expected non-empty proposal id, got %+v", createResp)
	}
	if createResp["status"] != "pending" {
		t.Errorf("expected status 'pending', got %q", createResp["status"])
	}

	// 2. GET /api/v1/learning/proposals (List Pending)
	reqList := httptest.NewRequest(http.MethodGet, "/api/v1/learning/proposals?status=pending", nil)
	recList := httptest.NewRecorder()
	server.ServeHTTP(recList, reqList)
	if recList.Code != http.StatusOK {
		t.Fatalf("expected 200 OK, got %d", recList.Code)
	}

	// 3. GET /api/v1/learning/proposals/{id} (Inspect single proposal)
	reqGet := httptest.NewRequest(http.MethodGet, "/api/v1/learning/proposals/"+propID, nil)
	recGet := httptest.NewRecorder()
	server.ServeHTTP(recGet, reqGet)
	if recGet.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from GET proposal, got %d", recGet.Code)
	}
	var propDetails memory.MemoryProposal
	if err := json.Unmarshal(recGet.Body.Bytes(), &propDetails); err != nil {
		t.Fatalf("failed to decode proposal details: %v", err)
	}
	if propDetails.Rule != reqBody.Rule {
		t.Errorf("expected rule %q, got %q", reqBody.Rule, propDetails.Rule)
	}
	if !propDetails.Reversible {
		t.Errorf("expected proposal to be reversible")
	}

	// 4. POST /api/v1/learning/proposals/{id}/approve (Approve -> writes scoped to memory)
	reqApprove := httptest.NewRequest(http.MethodPost, "/api/v1/learning/proposals/"+propID+"/approve", nil)
	recApprove := httptest.NewRecorder()
	server.ServeHTTP(recApprove, reqApprove)
	if recApprove.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from approve, got %d: %s", recApprove.Code, recApprove.Body.String())
	}

	// Verify rule is now written to scoped memory
	scopedResults, _ := vs.SearchWithScope(req.Context(), []float32{1}, 5, reqBody.Scope)
	if len(scopedResults) == 0 {
		t.Fatalf("expected rule written to scoped memory store after approval, got 0 results")
	}

	// 5. POST /api/v1/learning/proposals/{id}/revert (Revert -> removes written document)
	reqRevert := httptest.NewRequest(http.MethodPost, "/api/v1/learning/proposals/"+propID+"/revert", nil)
	recRevert := httptest.NewRecorder()
	server.ServeHTTP(recRevert, reqRevert)
	if recRevert.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from revert, got %d: %s", recRevert.Code, recRevert.Body.String())
	}

	// Verify rule is removed from memory after revert
	afterRevert, _ := vs.SearchWithScope(req.Context(), []float32{1}, 5, reqBody.Scope)
	if len(afterRevert) != 0 {
		t.Fatalf("expected rule removed from memory store after revert, got %d results", len(afterRevert))
	}

	// 6. Test Reject lifecycle for a separate proposal
	reqBody2 := approval.CreateProposalRequest{
		Rule: "Speculative unvetted rule to be rejected",
	}
	bodyBytes2, _ := json.Marshal(reqBody2)
	req2 := httptest.NewRequest(http.MethodPost, "/api/v1/learning/proposals", bytes.NewReader(bodyBytes2))
	rec2 := httptest.NewRecorder()
	server.ServeHTTP(rec2, req2)
	var createResp2 map[string]string
	_ = json.Unmarshal(rec2.Body.Bytes(), &createResp2)
	propID2 := createResp2["id"]

	reqReject := httptest.NewRequest(http.MethodPost, "/api/v1/learning/proposals/"+propID2+"/reject", nil)
	recReject := httptest.NewRecorder()
	server.ServeHTTP(recReject, reqReject)
	if recReject.Code != http.StatusOK {
		t.Fatalf("expected 200 OK from reject, got %d: %s", recReject.Code, recReject.Body.String())
	}

	// Verify rejected proposal is recorded as rejected and never written
	reqGet2 := httptest.NewRequest(http.MethodGet, "/api/v1/learning/proposals/"+propID2, nil)
	recGet2 := httptest.NewRecorder()
	server.ServeHTTP(recGet2, reqGet2)
	var propDetails2 memory.MemoryProposal
	_ = json.Unmarshal(recGet2.Body.Bytes(), &propDetails2)
	if propDetails2.Status != memory.ProposalRejected {
		t.Errorf("expected status 'rejected', got %q", propDetails2.Status)
	}
}
