package client

import (
	"context"
	"net"
	"testing"

	appErrors "github.com/diezy-labs/claw-crew/engine/core/errors"
	"github.com/diezy-labs/claw-crew/engine/pkg/pb"
	"google.golang.org/grpc"
)

type testVaultGatewayServer struct {
	pb.UnimplementedSystemGatewayServer
	secrets map[string]string
}

func (s *testVaultGatewayServer) GetDecryptedSecret(ctx context.Context, req *pb.SecretRequest) (*pb.SecretResponse, error) {
	val, ok := s.secrets[req.KeyName]
	if !ok {
		return &pb.SecretResponse{
			Found: false,
			Value: "",
		}, nil
	}
	return &pb.SecretResponse{
		Found: true,
		Value: val,
	}, nil
}

func TestSystemGatewayClient_GetDecryptedSecret(t *testing.T) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on random port: %v", err)
	}
	grpcServer := grpc.NewServer()
	mockSrv := &testVaultGatewayServer{
		secrets: map[string]string{
			"OPENAI_API_KEY":     "sk-test-secret-12345",
			"CLAWCREW_SIGN_KEY": "super-secure-token",
		},
	}
	pb.RegisterSystemGatewayServer(grpcServer, mockSrv)

	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	gwClient, err := NewSystemGatewayClient(lis.Addr().String())
	if err != nil {
		t.Fatalf("failed to create system gateway client: %v", err)
	}
	defer gwClient.Close()

	ctx := context.Background()

	// 1. Success case: Secret found in Rust vault
	val, err := gwClient.GetDecryptedSecret(ctx, "OPENAI_API_KEY")
	if err != nil {
		t.Fatalf("expected to fetch secret, got err: %v", err)
	}
	if val != "sk-test-secret-12345" {
		t.Errorf("expected secret value 'sk-test-secret-12345', got '%s'", val)
	}

	// 2. Not found case: Secret missing in Rust vault
	_, err = gwClient.GetDecryptedSecret(ctx, "NON_EXISTENT_KEY")
	if err == nil {
		t.Fatalf("expected error for non-existent secret, got nil")
	}
	appErr, ok := err.(*appErrors.AppError)
	if !ok {
		t.Fatalf("expected AppError, got %T: %v", err, err)
	}
	if appErr.Code != appErrors.CodeNotFound {
		t.Errorf("expected CodeNotFound, got %v", appErr.Code)
	}
}

func (s *testVaultGatewayServer) ExecuteTurn(req *pb.ExecuteTurnRequest, stream pb.SystemGateway_ExecuteTurnServer) error {
	// 1. Emit thought chunk
	if err := stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Token{
			Token: &pb.TokenChunk{
				Delta:     "Planning action...",
				IsThought: true,
			},
		},
	}); err != nil {
		return err
	}

	// 2. Emit text delta chunk
	if err := stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Token{
			Token: &pb.TokenChunk{
				Delta:     "Rust LLM SSOT Engine response: execution started.",
				IsThought: false,
			},
		},
	}); err != nil {
		return err
	}

	// 3. Emit turn complete
	return stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Complete{
			Complete: &pb.TurnComplete{
				FinishReason:     "stop",
				PromptTokens:     15,
				CompletionTokens: 25,
				TotalTokens:      40,
			},
		},
	})
}

func TestSystemGatewayClient_ExecuteTurn(t *testing.T) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on random port: %v", err)
	}
	grpcServer := grpc.NewServer()
	mockSrv := &testVaultGatewayServer{}
	pb.RegisterSystemGatewayServer(grpcServer, mockSrv)

	go func() {
		_ = grpcServer.Serve(lis)
	}()
	defer grpcServer.Stop()

	gwClient, err := NewSystemGatewayClient(lis.Addr().String())
	if err != nil {
		t.Fatalf("failed to create system gateway client: %v", err)
	}
	defer gwClient.Close()

	ctx := context.Background()
	stream, err := gwClient.ExecuteTurn(ctx, &pb.ExecuteTurnRequest{
		SessionId:    "session-integration-test",
		Model:        "gpt-4o",
		SystemPrompt: "You are an agent",
		Messages: []*pb.TurnChatMessage{
			{Role: "user", Content: "Start turn"},
		},
	})
	if err != nil {
		t.Fatalf("expected ExecuteTurn stream to start, got err: %v", err)
	}

	receivedThought := false
	receivedDelta := false
	receivedComplete := false

	for {
		token, err := stream.Recv()
		if err != nil {
			break
		}

		switch ev := token.Event.(type) {
		case *pb.TurnToken_Token:
			if ev.Token.IsThought {
				receivedThought = true
			} else {
				receivedDelta = true
			}
		case *pb.TurnToken_Complete:
			if ev.Complete.FinishReason == "stop" {
				receivedComplete = true
			}
		}
	}

	if !receivedThought {
		t.Errorf("expected to receive thought token from Rust gateway stream")
	}
	if !receivedDelta {
		t.Errorf("expected to receive content text token from Rust gateway stream")
	}
	if !receivedComplete {
		t.Errorf("expected to receive complete token from Rust gateway stream")
	}
}
