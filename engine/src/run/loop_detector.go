package run

// LoopDetectorStats represents loop detector telemetry for an agent turn (BL-16/BL-17)
// This structure mirrors the protobuf definition in proto/agent_service.proto
type LoopDetectorStats struct {
	// TotalCalls is the total number of tool calls observed
	TotalCalls int `json:"total_calls"`
	// ExactRepeatTriggers is the count of exact-repeat pattern triggers
	ExactRepeatTriggers int `json:"exact_repeat_triggers"`
	// PingPongTriggers is the count of ping-pong pattern triggers
	PingPongTriggers int `json:"ping_pong_triggers"`
	// NoProgressTriggers is the count of no-progress pattern triggers
	NoProgressTriggers int `json:"no_progress_triggers"`
	// State: 0=nominal, 1=warning, 2=block, 3=tripped
	State int `json:"state"`
}
