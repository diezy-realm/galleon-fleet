package mission

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestMissionService_CRUD(t *testing.T) {
	svc := NewService()
	ctx := context.Background()

	// Initial list should be empty
	quests, err := svc.List(ctx)
	if err != nil {
		t.Fatalf("List failed: %v", err)
	}
	if len(quests) != 0 {
		t.Fatalf("expected 0 quests, got %d", len(quests))
	}

	// Create a new quest
	q := &Quest{
		ID:        "quest-test-1",
		Title:     "Test Marketing Campaign Quest",
		Objective: "Coordinate social campaign launch",
		Priority:  "high",
		Status:    "ready",
		MapSteps: []QuestStep{
			{StepNumber: 1, Title: "Draft copy", Status: "pending"},
			{StepNumber: 2, Title: "Brand review", Status: "pending"},
		},
	}

	created, err := svc.Create(ctx, q)
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}
	if created.ID != "quest-test-1" {
		t.Errorf("expected quest-test-1, got %s", created.ID)
	}

	// Duplicate create should fail
	_, err = svc.Create(ctx, q)
	if err == nil {
		t.Fatal("expected duplicate create to fail, got nil")
	}

	// Get quest
	got, err := svc.Get(ctx, "quest-test-1")
	if err != nil {
		t.Fatalf("Get failed: %v", err)
	}
	if got.Title != "Test Marketing Campaign Quest" {
		t.Errorf("expected Title match, got %s", got.Title)
	}

	// Update status
	updated, err := svc.UpdateStatus(ctx, "quest-test-1", "underway")
	if err != nil {
		t.Fatalf("UpdateStatus failed: %v", err)
	}
	if updated.Status != "underway" {
		t.Errorf("expected status underway, got %s", updated.Status)
	}

	// Assign ship
	assigned, err := svc.AssignShip(ctx, "quest-test-1", "ship-growth")
	if err != nil {
		t.Fatalf("AssignShip failed: %v", err)
	}
	if assigned.AssignedShipID != "ship-growth" {
		t.Errorf("expected assigned ship-growth, got %s", assigned.AssignedShipID)
	}

	// Get non-existent quest should fail
	_, err = svc.Get(ctx, "non-existent")
	if err == nil {
		t.Fatal("expected Get non-existent to fail, got nil")
	}
}

func TestMission_QuestsJSONCompat(t *testing.T) {
	// Verify that real quests.json conforms to Quest struct definitions
	path := filepath.Join("..", "..", "data", "quests.json")
	bytes, err := os.ReadFile(path)
	if err != nil {
		t.Skipf("quests.json not found at %s: %v", path, err)
	}

	var quests []*Quest
	if err := json.Unmarshal(bytes, &quests); err != nil {
		t.Fatalf("failed to unmarshal quests.json into []*Quest: %v", err)
	}

	if len(quests) == 0 {
		t.Fatal("quests.json is empty")
	}

	first := quests[0]
	if first.ID == "" || first.Title == "" || first.Objective == "" {
		t.Errorf("quest fields missing: %+v", first)
	}
	if len(first.MapSteps) == 0 {
		t.Errorf("expected map steps in first quest: %+v", first)
	}

	// Test NewServiceWithQuests
	svc := NewServiceWithQuests(quests)
	loaded, err := svc.List(context.Background())
	if err != nil {
		t.Fatalf("List on pre-populated service failed: %v", err)
	}
	if len(loaded) != len(quests) {
		t.Errorf("expected %d quests loaded, got %d", len(quests), len(loaded))
	}
}
