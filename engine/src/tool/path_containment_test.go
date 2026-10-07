package tool

import (
	"context"
	"encoding/json"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"

	appErrors "github.com/diezy-realm/galleon-fleet/engine/core/errors"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/client"
	"github.com/diezy-realm/galleon-fleet/engine/pkg/pb"
	"google.golang.org/grpc"
)

func TestValidateSandboxPath_Traversal(t *testing.T) {
	tempDir := t.TempDir()
	workspaceRoot := filepath.Join(tempDir, "workspace")
	if err := os.MkdirAll(workspaceRoot, 0755); err != nil {
		t.Fatalf("failed to create workspace: %v", err)
	}

	validFile := filepath.Join(workspaceRoot, "hello.txt")
	if err := os.WriteFile(validFile, []byte("world"), 0644); err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	// 1. Valid path inside workspace
	res, err := ValidateSandboxPath(workspaceRoot, "hello.txt")
	if err != nil {
		t.Fatalf("expected valid path, got err: %v", err)
	}
	if res != validFile {
		t.Errorf("expected %s, got %s", validFile, res)
	}

	// 2. Traversal ../ outside workspace
	_, err = ValidateSandboxPath(workspaceRoot, "../outside.txt")
	if err == nil {
		t.Fatalf("expected error for ../ traversal, got nil")
	}
	if appErr, ok := err.(*appErrors.AppError); ok {
		if appErr.Code != appErrors.CodePermissionDenied {
			t.Errorf("expected CodePermissionDenied, got %v", appErr.Code)
		}
	}

	// 3. Sneaky relative traversal: sub/../../outside.txt
	_, err = ValidateSandboxPath(workspaceRoot, "sub/../../outside.txt")
	if err == nil {
		t.Fatalf("expected error for sneaky traversal, got nil")
	}

	// 4. Absolute path pointing outside workspace
	outsideFile := filepath.Join(tempDir, "outside.txt")
	_, err = ValidateSandboxPath(workspaceRoot, outsideFile)
	if err == nil {
		t.Fatalf("expected error for absolute path outside workspace, got nil")
	}
}

func TestValidateSandboxPath_SymlinkEscape(t *testing.T) {
	tempDir := t.TempDir()
	workspaceRoot := filepath.Join(tempDir, "workspace")
	outsideDir := filepath.Join(tempDir, "outside")
	if err := os.MkdirAll(workspaceRoot, 0755); err != nil {
		t.Fatalf("failed to create workspace: %v", err)
	}
	if err := os.MkdirAll(outsideDir, 0755); err != nil {
		t.Fatalf("failed to create outside dir: %v", err)
	}

	outsideTarget := filepath.Join(outsideDir, "secret.txt")
	if err := os.WriteFile(outsideTarget, []byte("confidential"), 0644); err != nil {
		t.Fatalf("failed to write outside file: %v", err)
	}

	symlinkPath := filepath.Join(workspaceRoot, "link_to_secret.txt")
	err := os.Symlink(outsideTarget, symlinkPath)
	if err != nil {
		t.Skipf("skipping symlink test (symlinks not permitted or supported): %v", err)
		return
	}

	// Attempt to access via symlink pointing outside
	_, err = ValidateSandboxPath(workspaceRoot, "link_to_secret.txt")
	if err == nil {
		t.Fatalf("expected error for symlink pointing outside workspace, got nil")
	}
	if appErr, ok := err.(*appErrors.AppError); ok {
		if appErr.Code != appErrors.CodePermissionDenied {
			t.Errorf("expected CodePermissionDenied, got %v", appErr.Code)
		}
	}
}

type testMockGatewayServer struct {
	pb.UnimplementedSystemGatewayServer
	workspaceRoot string
}

func (m *testMockGatewayServer) ExecuteNativeTool(ctx context.Context, req *pb.ToolCallRequest) (*pb.ToolCallResponse, error) {
	if req.ToolName != "read_file" && req.ToolName != "bash" && req.ToolName != "write_file" {
		return &pb.ToolCallResponse{
			Success: false,
			Error:   "tool outside allowlist",
		}, nil
	}

	var args struct {
		Path    string `json:"path"`
		Command string `json:"command"`
	}
	_ = json.Unmarshal([]byte(req.ArgumentsJson), &args)

	// Validate path containment if path specified
	if args.Path != "" {
		if _, err := ValidateSandboxPath(m.workspaceRoot, args.Path); err != nil {
			return &pb.ToolCallResponse{
				Success: false,
				Error:   "path traversal rejected: outside workspace root",
			}, nil
		}
	}

	// Validate path traversal attempts inside bash command
	if req.ToolName == "bash" && (strings.Contains(args.Command, "../") || strings.Contains(args.Command, "/etc/")) {
		return &pb.ToolCallResponse{
			Success: false,
			Error:   "path traversal rejected: command targets path outside workspace root",
		}, nil
	}

	return &pb.ToolCallResponse{
		Success: true,
		Output:  `{"status":"executed_in_isolated_sandbox","stdout":"success"}`,
	}, nil
}

func TestSystemGateway_PathContainment_GRPC(t *testing.T) {
	tempDir := t.TempDir()
	workspaceRoot := filepath.Join(tempDir, "workspace")
	if err := os.MkdirAll(workspaceRoot, 0755); err != nil {
		t.Fatalf("failed to create workspace: %v", err)
	}

	// 1. Start in-process gRPC SystemGateway server
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on ephemeral port: %v", err)
	}
	grpcServer := grpc.NewServer()
	mockSrv := &testMockGatewayServer{workspaceRoot: workspaceRoot}
	pb.RegisterSystemGatewayServer(grpcServer, mockSrv)

	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	// 2. Instantiate Go client
	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	if err != nil {
		t.Fatalf("failed to create gateway client: %v", err)
	}
	defer gwClient.Close()

	ctx := context.Background()

	// 3. Test round-trip with valid file inside workspace (T-4.2)
	validFile := filepath.Join(workspaceRoot, "safe.txt")
	if err := os.WriteFile(validFile, []byte("ok"), 0644); err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	output, err := gwClient.ExecuteNativeTool(ctx, "read_file", `{"path":"safe.txt"}`)
	if err != nil {
		t.Fatalf("expected successful roundtrip call via gRPC, got err: %v", err)
	}
	if !strings.Contains(output, "executed_in_isolated_sandbox") {
		t.Errorf("unexpected output from gateway: %s", output)
	}

	// 4. Test path traversal rejection via gRPC (T-4.3)
	_, err = gwClient.ExecuteNativeTool(ctx, "read_file", `{"path":"../../etc/passwd"}`)
	if err == nil {
		t.Fatalf("expected path traversal rejection over gRPC, got nil")
	}
	if !strings.Contains(err.Error(), "path traversal rejected") {
		t.Errorf("expected path traversal rejected message, got %v", err)
	}

	// 5. Test sneaky traversal in bash command via gRPC (T-4.3)
	_, err = gwClient.ExecuteNativeTool(ctx, "bash", `{"command":"cat ../../etc/shadow"}`)
	if err == nil {
		t.Fatalf("expected bash path traversal rejection over gRPC, got nil")
	}
	if !strings.Contains(err.Error(), "path traversal rejected") {
		t.Errorf("expected path traversal rejected message, got %v", err)
	}

	// 6. Test tool outside allowlist rejection via gRPC (T-4.2)
	_, err = gwClient.ExecuteNativeTool(ctx, "dangerous_unregistered_tool", `{}`)
	if err == nil {
		t.Fatalf("expected allowlist rejection over gRPC, got nil")
	}
	if !strings.Contains(err.Error(), "outside allowlist") {
		t.Errorf("expected tool outside allowlist message, got %v", err)
	}
}
