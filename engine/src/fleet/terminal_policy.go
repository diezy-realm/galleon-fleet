package fleet

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/diezy-realm/galleon-fleet/engine/core/logger"
)

type RiskTier string

const (
	RiskTierRead     RiskTier = "READ"
	RiskTierExecute  RiskTier = "EXECUTE"
	RiskTierHighRisk RiskTier = "HIGH_RISK"
	RiskTierBlocked  RiskTier = "BLOCKED"
)

type TerminalExecRequest struct {
	Command    string `json:"command"`
	Approved   bool   `json:"approved,omitempty"`
	ApprovedBy string `json:"approved_by,omitempty"`
}

type TerminalExecResponse struct {
	Stdout           string   `json:"stdout"`
	Stderr           string   `json:"stderr,omitempty"`
	ExitCode         int      `json:"exitCode"`
	Duration         string   `json:"duration"`
	RiskTier         RiskTier `json:"riskTier"`
	RequiresApproval bool     `json:"requiresApproval,omitempty"`
	AuditID          string   `json:"auditId,omitempty"`
	Error            string   `json:"error,omitempty"`
}

type TerminalAuditRecord struct {
	ID         string    `json:"id"`
	Timestamp  time.Time `json:"timestamp"`
	Command    string    `json:"command"`
	RiskTier   RiskTier  `json:"riskTier"`
	Approved   bool      `json:"approved"`
	ApprovedBy string    `json:"approvedBy,omitempty"`
	Success    bool      `json:"success"`
	ExitCode   int       `json:"exitCode"`
	Duration   string    `json:"duration"`
	Error      string    `json:"error,omitempty"`
}

// Prohibited / Injection patterns that must be blocked unconditionally (anti-bypass)
var (
	forkBombRegex    = regexp.MustCompile(`:\(\)\s*\{\s*:\|:&\s*\};:`)
	formatDiskRegex  = regexp.MustCompile(`(?i)\b(mkfs(\.[a-z0-9]+)?|format\s+[a-zA-Z]:|dd\s+if=/dev/)`)
	pipeToShellRegex = regexp.MustCompile(`(?i)\b(curl|wget)\b.*\|\s*(sh|bash|zsh|dash|powershell|cmd)\b`)
	highRiskRegex    = regexp.MustCompile(`(?i)\b(rm\s+-[a-zA-Z]*[rR][a-zA-Z]*|rmdir\b|del\b|git\s+reset\s+--hard|git\s+clean\s+-[a-zA-Z]*f|shutdown|reboot|init\s+0|kill\s+-9|taskkill|sudo|su\s+-)`)
	readCommands     = []string{
		"git status", "git diff", "git log", "git branch", "git show",
		"ls", "dir", "echo", "pwd", "whoami", "cat", "type",
		"head", "tail", "grep", "findstr", "ps", "which", "where", "uname",
	}
)

// EvaluateTerminalCommand analyzes a command string and classifies its RiskTier.
func EvaluateTerminalCommand(command string) (RiskTier, string, error) {
	trimmed := strings.TrimSpace(command)
	if trimmed == "" {
		return RiskTierBlocked, "", errors.New("command cannot be empty")
	}

	// 1. Prohibit null-bytes / null-character injections
	if strings.ContainsRune(trimmed, '\x00') {
		return RiskTierBlocked, "null byte injection detected", errors.New("command contains prohibited null-byte character")
	}

	// 2. Prohibit fork bombs and disk format commands
	if forkBombRegex.MatchString(trimmed) {
		return RiskTierBlocked, "fork bomb pattern detected", errors.New("command contains prohibited fork-bomb pattern")
	}
	if formatDiskRegex.MatchString(trimmed) {
		return RiskTierBlocked, "destructive disk format command detected", errors.New("command contains prohibited disk format pattern")
	}

	// 3. High Risk checks (untrusted shell pipe or destructive ops)
	if pipeToShellRegex.MatchString(trimmed) {
		return RiskTierHighRisk, "untrusted remote script pipe to shell requires explicit approval", nil
	}
	if highRiskRegex.MatchString(trimmed) {
		return RiskTierHighRisk, "destructive or privileged operation requires explicit approval", nil
	}

	// 4. Read-only checks
	lowerCmd := strings.ToLower(trimmed)
	for _, readPrefix := range readCommands {
		if lowerCmd == readPrefix || strings.HasPrefix(lowerCmd, readPrefix+" ") {
			return RiskTierRead, "read-only diagnostic/inspection command", nil
		}
	}

	// 5. Default to EXECUTE for standard developer tooling
	return RiskTierExecute, "standard developer tool execution", nil
}

// TerminalAuditLedger maintains an in-memory thread-safe ledger of terminal executions
type TerminalAuditLedger struct {
	mu      sync.RWMutex
	records []TerminalAuditRecord
	maxSize int
}

func NewTerminalAuditLedger(maxSize int) *TerminalAuditLedger {
	if maxSize <= 0 {
		maxSize = 1000
	}
	return &TerminalAuditLedger{
		records: make([]TerminalAuditRecord, 0, maxSize),
		maxSize: maxSize,
	}
}

func (l *TerminalAuditLedger) Record(rec TerminalAuditRecord) string {
	l.mu.Lock()
	defer l.mu.Unlock()

	if rec.ID == "" {
		hash := sha256.Sum256([]byte(fmt.Sprintf("%d:%s:%s", rec.Timestamp.UnixNano(), rec.Command, rec.RiskTier)))
		rec.ID = fmt.Sprintf("audit-%s", hex.EncodeToString(hash[:8]))
	}

	if len(l.records) >= l.maxSize {
		// Evict oldest record
		l.records = l.records[1:]
	}
	l.records = append(l.records, rec)

	logger.Get().Info("Terminal audit record logged",
		"id", rec.ID,
		"command", rec.Command,
		"riskTier", string(rec.RiskTier),
		"approved", rec.Approved,
		"exitCode", rec.ExitCode,
	)

	return rec.ID
}

func (l *TerminalAuditLedger) List(limit int) []TerminalAuditRecord {
	l.mu.RLock()
	defer l.mu.RUnlock()

	if limit <= 0 || limit > len(l.records) {
		limit = len(l.records)
	}

	start := len(l.records) - limit
	result := make([]TerminalAuditRecord, limit)
	copy(result, l.records[start:])
	return result
}
