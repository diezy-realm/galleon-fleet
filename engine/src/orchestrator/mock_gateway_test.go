package orchestrator_test

import (
	"net"
	"testing"

	"github.com/diezy-labs/claw-crew/engine/pkg/client"
	"github.com/diezy-labs/claw-crew/engine/pkg/pb"
	"google.golang.org/grpc"
)

type mockOrchGatewayServer struct {
	pb.UnimplementedSystemGatewayServer
	response string
}

func (s *mockOrchGatewayServer) ExecuteTurn(req *pb.ExecuteTurnRequest, stream pb.SystemGateway_ExecuteTurnServer) error {
	return stream.Send(&pb.TurnToken{
		Event: &pb.TurnToken_Token{
			Token: &pb.TokenChunk{
				Delta:     s.response,
				IsThought: false,
			},
		},
	})
}

func startMockGateway(t *testing.T, response string) (client.SystemGatewayClient, func()) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	srv := grpc.NewServer()
	pb.RegisterSystemGatewayServer(srv, &mockOrchGatewayServer{response: response})
	go func() { _ = srv.Serve(lis) }()

	gwClient, err := client.NewSystemGatewayClient(lis.Addr().String())
	if err != nil {
		srv.Stop()
		t.Fatalf("client: %v", err)
	}
	return gwClient, func() {
		_ = gwClient.Close()
		srv.Stop()
	}
}
