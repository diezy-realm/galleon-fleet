package crew

import (
	"strings"
	"unicode"
)

// StandardTokenCounter implements TokenCounter using standard LLM tokenization heuristics
// and canonical model pricing catalogs.
type StandardTokenCounter struct{}

// NewStandardTokenCounter constructs a new token counter instance
func NewStandardTokenCounter() TokenCounter {
	return &StandardTokenCounter{}
}

// CountTokens estimates the number of tokens in a string using tokenization heuristics
func (c *StandardTokenCounter) CountTokens(text string) int {
	if len(text) == 0 {
		return 0
	}

	// Fast path for short tokens
	if len(text) <= 4 {
		return 1
	}

	// Hybrid heuristic: count words, punctuation, and character lengths
	words := 0
	puncts := 0
	inWord := false

	for _, r := range text {
		if unicode.IsSpace(r) {
			inWord = false
		} else if unicode.IsPunct(r) || unicode.IsSymbol(r) {
			inWord = false
			puncts++
		} else {
			if !inWord {
				words++
				inWord = true
			}
		}
	}

	// Most tokenizers tokenize 1 word to ~1.3 tokens, plus punctuation tokens
	wordBasedEstimate := int(float64(words)*1.33) + puncts
	charBasedEstimate := (len(text) + 3) / 4

	// Average of character heuristic and word/punct heuristic for balanced accuracy
	tokens := (wordBasedEstimate + charBasedEstimate) / 2
	if tokens < 1 {
		tokens = 1
	}
	return tokens
}

// EstimateMessageTokens estimates token overhead and content tokens for a single chat message
func (c *StandardTokenCounter) EstimateMessageTokens(msg ChatMessage) int {
	// Standard per-message formatting overhead: <|im_start|>{role}\n ... <|im_end|>\n = 4 tokens
	tokens := 4
	tokens += c.CountTokens(msg.Content)

	if msg.Name != "" {
		tokens += c.CountTokens(msg.Name) + 1
	}
	if msg.ToolCallID != "" {
		tokens += c.CountTokens(msg.ToolCallID) + 2
	}
	return tokens
}

// EstimateContextTokens estimates total prompt tokens for an assembled ResolvedContext
func (c *StandardTokenCounter) EstimateContextTokens(resolved *ResolvedContext) int {
	if resolved == nil {
		return 0
	}

	tokens := 0
	if resolved.SystemPrompt != "" {
		tokens += 4 + c.CountTokens(resolved.SystemPrompt)
	}

	for _, msg := range resolved.Messages {
		tokens += c.EstimateMessageTokens(msg)
	}

	for _, tool := range resolved.Tools {
		// Tool schema definition tokens
		tokens += 8 + c.CountTokens(tool.Name) + c.CountTokens(tool.Description) + c.CountTokens(tool.ParametersJSON)
	}

	// Priming token for assistant generation: <|im_start|>assistant\n = 3 tokens
	tokens += 3
	return tokens
}

// CalculateCost computes total USD cost based on token counts and model pricing
func (c *StandardTokenCounter) CalculateCost(promptTokens, completionTokens int, model string) float64 {
	m := strings.ToLower(model)

	var promptRatePerMillion, completionRatePerMillion float64

	switch {
	case strings.Contains(m, "claude-3-5-sonnet") || strings.Contains(m, "gpt-4o"):
		promptRatePerMillion = 2.50
		completionRatePerMillion = 10.00
	case strings.Contains(m, "claude-3-opus"):
		promptRatePerMillion = 15.00
		completionRatePerMillion = 75.00
	case strings.Contains(m, "flash") || strings.Contains(m, "mini"):
		promptRatePerMillion = 0.15
		completionRatePerMillion = 0.60
	case strings.Contains(m, "gpt-4") || strings.Contains(m, "claude-3"):
		promptRatePerMillion = 3.00
		completionRatePerMillion = 15.00
	default:
		// Default / fallback pricing
		promptRatePerMillion = 1.00
		completionRatePerMillion = 2.00
	}

	promptCost := (float64(promptTokens) / 1_000_000.0) * promptRatePerMillion
	completionCost := (float64(completionTokens) / 1_000_000.0) * completionRatePerMillion

	return promptCost + completionCost
}

// Reconcile reconciles estimated token usage with actual provider reporting
func (c *StandardTokenCounter) Reconcile(estimated, actual TokenUsage) TokenUsage {
	if actual.TotalTokens > 0 {
		cost := actual.CostUSD
		if cost == 0 {
			cost = c.CalculateCost(actual.PromptTokens, actual.CompletionTokens, "")
		}
		return TokenUsage{
			PromptTokens:     actual.PromptTokens,
			CompletionTokens: actual.CompletionTokens,
			TotalTokens:      actual.TotalTokens,
			CostUSD:          cost,
		}
	}

	total := estimated.PromptTokens + estimated.CompletionTokens
	if estimated.TotalTokens > total {
		total = estimated.TotalTokens
	}
	return TokenUsage{
		PromptTokens:     estimated.PromptTokens,
		CompletionTokens: estimated.CompletionTokens,
		TotalTokens:      total,
		CostUSD:          estimated.CostUSD,
	}
}
