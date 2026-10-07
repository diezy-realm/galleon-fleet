package fleet_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/core/metrics"
	"github.com/diezy-realm/galleon-fleet/engine/src/entitlement"
	"github.com/diezy-realm/galleon-fleet/engine/src/fleet"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestEntitlementUsageEndpointAndQuotaGuard(t *testing.T) {
	t.Setenv("GALLEON_DATA_DIR", t.TempDir())
	svc := fleet.NewService(nil)
	handler := fleet.NewHTTPHandler(svc)
	server := metrics.NewServer(0)
	handler.RegisterHTTP(server)

	// 1. GET /api/v1/entitlement/usage
	req := httptest.NewRequest(http.MethodGet, "/api/v1/entitlement/usage", nil)
	w := httptest.NewRecorder()
	server.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	var report entitlement.UsageReport
	err := json.Unmarshal(w.Body.Bytes(), &report)
	require.NoError(t, err)
	assert.Equal(t, entitlement.TierCommunity, report.Tier)
	assert.Equal(t, 1, report.MaxShips)
	assert.Equal(t, 2, report.MaxSquadsTotal)
	require.NotNil(t, report.DelegationDepthCap)
	assert.Equal(t, 1, *report.DelegationDepthCap)

	// 2. Test POST /api/collections/ships with 1 ship -> allowed on Community
	oneShipJSON := `[{"id":"shp_1","name":"Interceptor"}]`
	req = httptest.NewRequest(http.MethodPost, "/api/collections/ships", bytes.NewBufferString(oneShipJSON))
	w = httptest.NewRecorder()
	server.ServeHTTP(w, req)
	assert.Equal(t, http.StatusOK, w.Code)

	// 3. Test POST /api/collections/ships with 2 ships -> rejected on Community (MaxShips: 1)
	twoShipsJSON := `[{"id":"shp_1","name":"Interceptor"},{"id":"shp_2","name":"Dreadnought"}]`
	req = httptest.NewRequest(http.MethodPost, "/api/collections/ships", bytes.NewBufferString(twoShipsJSON))
	w = httptest.NewRecorder()
	server.ServeHTTP(w, req)
	assert.Equal(t, http.StatusInternalServerError, w.Code)
	assert.Contains(t, w.Body.String(), "quota exceeded for ships")

	// 4. Upgrade to Team policy -> saving 2 ships now succeeds
	svc.SetEntitlementPolicy(entitlement.NewTeamPolicy())
	req = httptest.NewRequest(http.MethodPost, "/api/collections/ships", bytes.NewBufferString(twoShipsJSON))
	w = httptest.NewRecorder()
	server.ServeHTTP(w, req)
	assert.Equal(t, http.StatusOK, w.Code)
}
