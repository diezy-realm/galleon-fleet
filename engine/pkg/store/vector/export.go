package vector

import (
	"bytes"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

// ExportDocuments serializes documents into the requested format (json/csv)
func ExportDocuments(docs []*TimedDocument, req ExportRequest) ([]byte, error) {
	filtered := make([]*TimedDocument, 0, len(docs))
	for _, d := range docs {
		if d == nil || d.Document == nil {
			continue
		}
		// Scope check
		if !matchStrictScope(d.Document.Scope, req.Scope) {
			continue
		}
		// Time range check
		if !req.FromTime.IsZero() && d.CreatedAt.Before(req.FromTime) {
			continue
		}
		if !req.ToTime.IsZero() && d.CreatedAt.After(req.ToTime) {
			continue
		}
		filtered = append(filtered, d)
	}

	format := strings.ToLower(strings.TrimSpace(req.Format))
	if format == "" || format == "json" {
		return json.MarshalIndent(filtered, "", "  ")
	}

	if format == "csv" {
		var buf bytes.Buffer
		writer := csv.NewWriter(&buf)
		// Header
		if err := writer.Write([]string{"id", "content", "created_at", "scope"}); err != nil {
			return nil, err
		}

		for _, d := range filtered {
			scopeBytes, _ := json.Marshal(d.Document.Scope)
			row := []string{
				d.Document.ID,
				d.Document.Content,
				d.CreatedAt.Format(time.RFC3339),
				string(scopeBytes),
			}
			if err := writer.Write(row); err != nil {
				return nil, err
			}
		}
		writer.Flush()
		return buf.Bytes(), writer.Error()
	}

	return nil, fmt.Errorf("unsupported export format: %s", req.Format)
}
