package client

import (
	"context"
	"encoding/json"
	"log/slog"
	"strings"
	"sync"
	"time"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/core/logger"
	"github.com/diezy-realm/galleon-fleet/engine/core/tracing"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
)

// withRequestIDMetadata propagates x-request-id from Go context to outgoing gRPC metadata for Rust tracing (BG-F5)
func withRequestIDMetadata(ctx context.Context) context.Context {
	if reqID, ok := ctx.Value("request_id").(string); ok && reqID != "" {
		return metadata.AppendToOutgoingContext(ctx, "x-request-id", reqID)
	}
	if traceID, ok := ctx.Value("trace_id").(string); ok && traceID != "" {
		return metadata.AppendToOutgoingContext(ctx, "x-request-id", traceID)
	}
	if span := tracing.SpanFromContext(ctx); span != nil && span.TraceID != "" {
		return metadata.AppendToOutgoingContext(ctx, "x-request-id", span.TraceID)
	}
	return ctx
}

// SystemGatewayClient defines the interface for calling Rust platform operations
type SystemGatewayClient interface {
	ExecuteNativeTool(ctx context.Context, toolName, argumentsJSON string, workspacePath ...string) (string, error)
	GetNativeToolCatalog(ctx context.Context) ([]*pb.NativeToolDefinition, error)
	GetDecryptedSecret(ctx context.Context, keyName string) (string, error)
	ExecuteTask(ctx context.Context, taskID, taskData string) (string, error)
	ExecuteTurn(ctx context.Context, req *pb.ExecuteTurnRequest) (pb.SystemGateway_ExecuteTurnClient, error)
	StoreLearnedRule(ctx context.Context, rule string, scope map[string]string, reversible bool) (*pb.StoreLearnedRuleResponse, error)
	RetrieveLearnedRules(ctx context.Context, scopeKey, scopeValue string) ([]*pb.LearnedRule, error)
	SyncPairedDevice(ctx context.Context, devID, devName, platform, tokenHash string) error
	ListPairedDevices(ctx context.Context) ([]*pb.DeviceRecord, error)
	RevokePairedDevice(ctx context.Context, devID string) error
	Close() error
}

type systemGatewayClient struct {
	addr       string
	mu         sync.Mutex
	conn       *grpc.ClientConn
	client     pb.SystemGatewayClient
	lastFailed time.Time
}

// NewSystemGatewayClient creates a new client connected to Rust's SystemGateway
func NewSystemGatewayClient(addr string) (SystemGatewayClient, error) {
	if addr == "" {
		addr = "localhost:50052"
	}

	return &systemGatewayClient{
		addr: addr,
	}, nil
}

func (c *systemGatewayClient) getClient() (pb.SystemGatewayClient, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.client != nil {
		return c.client, nil
	}

	// Avoid repeated blocking dial attempts when host is offline
	if time.Since(c.lastFailed) < 3*time.Second {
		return nil, appErrors.New(appErrors.CodeUnavailable, "system gateway currently unavailable", appErrors.LayerExternal)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 400*time.Millisecond)
	defer cancel()

	conn, err := grpc.DialContext(
		ctx,
		c.addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		c.lastFailed = time.Now()
		// Log warning but return lazily connectable client structure
		logger.Get().Warn("system gateway connection delayed",
			slog.String("address", c.addr),
			slog.String("error", err.Error()),
		)
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to connect to Rust SystemGateway", appErrors.LayerExternal)
	}

	c.conn = conn
	c.client = pb.NewSystemGatewayClient(conn)
	return c.client, nil
}

func (c *systemGatewayClient) ExecuteNativeTool(ctx context.Context, toolName, argumentsJSON string, workspacePath ...string) (string, error) {
	log := logger.Get()
	log.InfoContext(ctx, "executing native tool via Rust gateway",
		slog.String("tool", toolName),
	)

	client, err := c.getClient()
	if err != nil {
		log.ErrorContext(ctx, "failed to get Rust SystemGateway client, rejecting execution",
			slog.String("tool", toolName),
			slog.String("error", err.Error()),
		)
		return "", appErrors.Wrap(err, appErrors.CodeUnavailable, "Rust SystemGateway unreachable: native tool execution rejected (fail-closed boundary enforced)", appErrors.LayerExternal)
	}

	var wsPath *string
	if len(workspacePath) > 0 && strings.TrimSpace(workspacePath[0]) != "" {
		ws := workspacePath[0]
		wsPath = &ws
	} else if strings.TrimSpace(argumentsJSON) != "" {
		var env map[string]any
		if err := json.Unmarshal([]byte(argumentsJSON), &env); err == nil {
			if ws, ok := env["__workspace_root"].(string); ok && strings.TrimSpace(ws) != "" {
				wsPath = &ws
			} else if ws, ok := env["workspace_path"].(string); ok && strings.TrimSpace(ws) != "" {
				wsPath = &ws
			}
		}
	}

	ctx = withRequestIDMetadata(ctx)
	resp, err := client.ExecuteNativeTool(ctx, &pb.ToolCallRequest{
		ToolName:      toolName,
		ArgumentsJson: argumentsJSON,
		WorkspacePath: wsPath,
	})
	if err != nil {
		return "", appErrors.Wrap(err, appErrors.CodeToolFailed, "Rust gateway tool execution error", appErrors.LayerExternal)
	}

	if !resp.GetSuccess() {
		return "", appErrors.New(appErrors.CodeToolFailed, resp.GetError(), appErrors.LayerExternal)
	}

	return resp.GetOutput(), nil
}

func (c *systemGatewayClient) GetNativeToolCatalog(ctx context.Context) ([]*pb.NativeToolDefinition, error) {
	client, err := c.getClient()
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeUnavailable, "Rust SystemGateway unreachable", appErrors.LayerExternal)
	}

	ctx = withRequestIDMetadata(ctx)
	resp, err := client.GetNativeToolCatalog(ctx, &pb.GetNativeToolCatalogRequest{})
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeToolFailed, "failed to get native tool catalog", appErrors.LayerExternal)
	}
	return resp.GetTools(), nil
}

func (c *systemGatewayClient) GetDecryptedSecret(ctx context.Context, keyName string) (string, error) {
	client, err := c.getClient()
	if err != nil {
		return "", err
	}

	ctx = withRequestIDMetadata(ctx)
	resp, err := client.GetDecryptedSecret(ctx, &pb.SecretRequest{
		KeyName: keyName,
	})
	if err != nil {
		return "", appErrors.Wrap(err, appErrors.CodeInternal, "failed to fetch secret from vault", appErrors.LayerExternal)
	}

	if !resp.GetFound() {
		return "", appErrors.New(appErrors.CodeNotFound, "secret key not found in vault", appErrors.LayerExternal)
	}

	return resp.GetValue(), nil
}

func (c *systemGatewayClient) ExecuteTask(ctx context.Context, taskID, taskData string) (string, error) {
	client, err := c.getClient()
	if err != nil {
		return "", err
	}

	ctx = withRequestIDMetadata(ctx)
	resp, err := client.ExecuteTask(ctx, &pb.TaskExecutionRequest{
		TaskId:   taskID,
		TaskData: taskData,
	})
	if err != nil {
		return "", appErrors.Wrap(err, appErrors.CodeInternal, "failed to execute task via Rust gateway", appErrors.LayerExternal)
	}

	if !resp.GetSuccess() {
		return "", appErrors.New(appErrors.CodeInternal, resp.GetError(), appErrors.LayerExternal)
	}

	return resp.GetOutput(), nil
}

func (c *systemGatewayClient) ExecuteTurn(ctx context.Context, req *pb.ExecuteTurnRequest) (pb.SystemGateway_ExecuteTurnClient, error) {
	client, err := c.getClient()
	if err != nil {
		return nil, err
	}

	ctx = withRequestIDMetadata(ctx)
	stream, err := client.ExecuteTurn(ctx, req)
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to start ExecuteTurn streaming via Rust gateway", appErrors.LayerExternal)
	}

	return stream, nil
}

func (c *systemGatewayClient) StoreLearnedRule(ctx context.Context, rule string, scope map[string]string, reversible bool) (*pb.StoreLearnedRuleResponse, error) {
	client, err := c.getClient()
	if err != nil {
		return nil, err
	}

	resp, err := client.StoreLearnedRule(ctx, &pb.StoreLearnedRuleRequest{
		Rule:       rule,
		Scope:      scope,
		Reversible: reversible,
	})
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to store learned rule via Rust gateway", appErrors.LayerExternal)
	}

	return resp, nil
}

func (c *systemGatewayClient) RetrieveLearnedRules(ctx context.Context, scopeKey, scopeValue string) ([]*pb.LearnedRule, error) {
	client, err := c.getClient()
	if err != nil {
		return nil, err
	}

	resp, err := client.RetrieveLearnedRules(ctx, &pb.RetrieveLearnedRulesRequest{
		ScopeKey:   scopeKey,
		ScopeValue: scopeValue,
	})
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to retrieve learned rules via Rust gateway", appErrors.LayerExternal)
	}

	return resp.GetRules(), nil
}

func (c *systemGatewayClient) SyncPairedDevice(ctx context.Context, devID, devName, platform, tokenHash string) error {
	client, err := c.getClient()
	if err != nil {
		return err
	}

	resp, err := client.SyncPairedDevice(ctx, &pb.SyncPairedDeviceRequest{
		DeviceId:   devID,
		DeviceName: devName,
		Platform:   platform,
		TokenHash:  tokenHash,
	})
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to sync paired device to Rust registry", appErrors.LayerExternal)
	}

	if !resp.GetSuccess() {
		return appErrors.New(appErrors.CodeInternal, resp.GetError(), appErrors.LayerExternal)
	}

	return nil
}

func (c *systemGatewayClient) ListPairedDevices(ctx context.Context) ([]*pb.DeviceRecord, error) {
	client, err := c.getClient()
	if err != nil {
		return nil, err
	}

	resp, err := client.ListPairedDevices(ctx, &pb.ListPairedDevicesRequest{})
	if err != nil {
		return nil, appErrors.Wrap(err, appErrors.CodeInternal, "failed to list paired devices from Rust registry", appErrors.LayerExternal)
	}

	return resp.GetDevices(), nil
}

func (c *systemGatewayClient) RevokePairedDevice(ctx context.Context, devID string) error {
	client, err := c.getClient()
	if err != nil {
		return err
	}

	resp, err := client.RevokePairedDevice(ctx, &pb.RevokePairedDeviceRequest{
		DeviceId: devID,
	})
	if err != nil {
		return appErrors.Wrap(err, appErrors.CodeInternal, "failed to revoke paired device in Rust registry", appErrors.LayerExternal)
	}

	if !resp.GetSuccess() {
		return appErrors.New(appErrors.CodeInternal, resp.GetError(), appErrors.LayerExternal)
	}

	return nil
}

func (c *systemGatewayClient) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.conn != nil {
		err := c.conn.Close()
		c.conn = nil
		c.client = nil
		return err
	}
	return nil
}
