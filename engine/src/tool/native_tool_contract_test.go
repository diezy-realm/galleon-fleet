package tool

import (
	"context"
	"encoding/json"
	"net"
	"testing"

	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"google.golang.org/grpc"
)

// mockContractGatewayServer records received requests for contract validation
type mockContractGatewayServer struct {
	pb.UnimplementedSystemGatewayServer
	lastRequest *pb.ToolCallRequest
}

func (s *mockContractGatewayServer) ExecuteNativeTool(ctx context.Context, req *pb.ToolCallRequest) (*pb.ToolCallResponse, error) {
	s.lastRequest = req

	// Validate write_file contract specifically
	if req.ToolName == "write_file" {
		var args map[string]any
		if err := json.Unmarshal([]byte(req.ArgumentsJson), &args); err != nil {
			return &pb.ToolCallResponse{Success: false, Error: "invalid json"}, nil
		}
		// In SSOT, content must be present (not contents)
		if _, ok := args["content"]; !ok {
			return &pb.ToolCallResponse{
				Success: false,
				Error:   "contract violation: write_file requires 'content' key in arguments",
			}, nil
		}
		if _, ok := args["path"]; !ok {
			return &pb.ToolCallResponse{
				Success: false,
				Error:   "contract violation: write_file requires 'path' key in arguments",
			}, nil
		}
		return &pb.ToolCallResponse{
			Success: true,
			Output:  "wrote bytes to " + args["path"].(string),
		}, nil
	}

	return &pb.ToolCallResponse{
		Success: true,
		Output:  "executed " + req.ToolName,
	}, nil
}

func (s *mockContractGatewayServer) GetNativeToolCatalog(ctx context.Context, req *pb.GetNativeToolCatalogRequest) (*pb.GetNativeToolCatalogResponse, error) {
	return &pb.GetNativeToolCatalogResponse{
		Tools: []*pb.NativeToolDefinition{
			{
				Name:            "write_file",
				Description:     "Write content to a file inside workspace",
				RiskTier:        "write",
				InputSchemaJson: `{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}`,
				Aliases:         []string{},
			},
			{
				Name:            "read_file",
				Description:     "Read contents of a file inside workspace",
				RiskTier:        "read",
				InputSchemaJson: `{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}`,
				Aliases:         []string{"workspace.read_file"},
			},
			{
				Name:            "bash",
				Description:     "Execute shell command",
				RiskTier:        "execute",
				InputSchemaJson: `{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}`,
				Aliases:         []string{"execute_command"},
			},
			{
				Name:            "git",
				Description:     "Execute git commands",
				RiskTier:        "execute",
				InputSchemaJson: `{"type":"object","properties":{"args":{"type":"string"}},"required":["args"]}`,
				Aliases:         []string{"git.status", "git.commit"},
			},
		},
	}, nil
}

func TestNativeToolSSOT_WriteFileContentSchema(t *testing.T) {
	// Verify Go WriteFileTool uses 'content' in its schema, matching SSOT
	tool := &WriteFileTool{}
	def := tool.Definition()
	require.NotNil(t, def)

	var schema map[string]any
	err := json.Unmarshal(def.InputSchema, &schema)
	require.NoError(t, err)

	props, ok := schema["properties"].(map[string]any)
	require.True(t, ok)
	assert.Contains(t, props, "content", "WriteFileTool must use 'content' in schema properties")
	assert.NotContains(t, props, "contents", "WriteFileTool must NOT use 'contents' in schema properties")

	reqs, ok := schema["required"].([]any)
	require.True(t, ok)
	assert.Contains(t, reqs, "content")
	assert.Contains(t, reqs, "path")
}

func TestNativeToolSSOT_WorkspacePathPopulated(t *testing.T) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	require.NoError(t, err)
	defer lis.Close()

	mockSrv := &mockContractGatewayServer{}
	grpcServer := grpc.NewServer()
	pb.RegisterSystemGatewayServer(grpcServer, mockSrv)

	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	require.NoError(t, err)
	defer gwClient.Close()

	ctx := context.Background()
	workspaceRoot := "/tmp/test-workspace"

	// 1. Test ExecuteNativeTool with workspaceRoot parameter
	out, err := gwClient.ExecuteNativeTool(ctx, "write_file", `{"path":"foo.txt","content":"bar"}`, workspaceRoot)
	require.NoError(t, err)
	assert.Contains(t, out, "wrote bytes to foo.txt")

	require.NotNil(t, mockSrv.lastRequest)
	require.NotNil(t, mockSrv.lastRequest.WorkspacePath, "workspace_path must be populated in ToolCallRequest")
	assert.Equal(t, workspaceRoot, *mockSrv.lastRequest.WorkspacePath)

	// 2. Test ExecuteNativeTool with __workspace_root in JSON arguments
	out, err = gwClient.ExecuteNativeTool(ctx, "write_file", `{"path":"foo.txt","content":"bar","__workspace_root":"/custom/path"}`)
	require.NoError(t, err)
	assert.Contains(t, out, "wrote bytes to foo.txt")

	require.NotNil(t, mockSrv.lastRequest.WorkspacePath)
	assert.Equal(t, "/custom/path", *mockSrv.lastRequest.WorkspacePath)
}

func TestNativeToolSSOT_CatalogQuery(t *testing.T) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	require.NoError(t, err)
	defer lis.Close()

	mockSrv := &mockContractGatewayServer{}
	grpcServer := grpc.NewServer()
	pb.RegisterSystemGatewayServer(grpcServer, mockSrv)

	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	require.NoError(t, err)
	defer gwClient.Close()

	ctx := context.Background()
	catalog, err := gwClient.GetNativeToolCatalog(ctx)
	require.NoError(t, err)
	require.Len(t, catalog, 4)

	var writeTool *pb.NativeToolDefinition
	for _, tool := range catalog {
		if tool.Name == "write_file" {
			writeTool = tool
			break
		}
	}
	require.NotNil(t, writeTool)
	assert.Contains(t, writeTool.InputSchemaJson, "content")
}
